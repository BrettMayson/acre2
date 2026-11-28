//! Converts a WAV file into a base64 sound file (`.b64`) for `CfgAcreSounds`.
//!
//! The input must be mono, 16-bit PCM, 48 kHz. Usage: `wav2b64 input.wav [output.b64]`.
//! Replaces the C++ `Wav2B64.exe` with the same output format.

use std::path::PathBuf;

use anyhow::{Context, bail};
use base64::Engine;

/// Channels, sample rate and bits per sample from the `fmt ` chunk.
fn format(wav: &[u8]) -> anyhow::Result<(u16, u32, u16)> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        bail!("not a WAV file");
    }
    let mut pos = 12;
    while pos + 8 <= wav.len() {
        let id = &wav[pos..pos + 4];
        let size = u32::from_le_bytes(wav[pos + 4..pos + 8].try_into()?) as usize;
        let body = wav
            .get(pos + 8..pos + 8 + size)
            .context("truncated WAV chunk")?;
        if id == b"fmt " {
            if body.len() < 16 {
                bail!("short fmt chunk");
            }
            let format = u16::from_le_bytes([body[0], body[1]]);
            if format != 1 {
                bail!("input must be PCM (format {format})");
            }
            return Ok((
                u16::from_le_bytes([body[2], body[3]]),
                u32::from_le_bytes(body[4..8].try_into()?),
                u16::from_le_bytes([body[14], body[15]]),
            ));
        }
        pos += 8 + size + (size & 1);
    }
    bail!("no fmt chunk")
}

/// The `.b64` text: `ACRE_B64_FILE = [ "<2048 chars>", …, "" ];`, indented with four spaces
/// like the repo's sounds (the C++ used tabs).
fn encode(wav: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(wav);
    let mut out = String::from("ACRE_B64_FILE = [\n");
    // As the C++: chunks 0..=len/2048 (an empty last chunk when it divides evenly).
    for x in 0..=b64.len() / 2048 {
        let chunk = b64
            .get(x * 2048..((x + 1) * 2048).min(b64.len()))
            .unwrap_or("");
        out.push_str("    \"");
        out.push_str(chunk);
        out.push_str("\",\n");
    }
    out.push_str("    \"\"\n];");
    out
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(input) = args.first().map(PathBuf::from) else {
        println!(
            "Converts WAV files into base64 encoded sound files.\n\nUsage: wav2b64 input.wav [output.b64]"
        );
        return Ok(());
    };
    if !input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
    {
        bail!("input must be a .wav file: {}", input.display());
    }
    let output = args
        .get(1)
        .map_or_else(|| input.with_extension("b64"), PathBuf::from);
    let wav = std::fs::read(&input).with_context(|| format!("reading {}", input.display()))?;
    let (channels, rate, bits) = format(&wav)?;
    if channels > 1 {
        bail!("input must be a mono channel PCM WAV file");
    }
    if bits != 16 {
        bail!("input must be a 16-bit PCM WAV file");
    }
    if rate != 48_000 {
        bail!("input must have a sample rate of 48 kHz");
    }
    std::fs::write(&output, encode(&wav))
        .with_context(|| format!("writing {}", output.display()))?;
    println!("{} → {}", input.display(), output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(channels: u16, rate: u32, bits: u16, samples: usize) -> Vec<u8> {
        let data = vec![0u8; samples * 2];
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&u32::try_from(36 + data.len()).unwrap().to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&bits.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        v.extend_from_slice(&data);
        v
    }

    #[test]
    fn reads_format_and_encodes() {
        let w = wav(1, 48_000, 16, 3000);
        assert_eq!(format(&w).unwrap(), (1, 48_000, 16));
        assert_eq!(format(&wav(2, 44_100, 16, 10)).unwrap(), (2, 44_100, 16));
        assert!(format(b"nope").is_err());
        let text = encode(&w);
        assert!(text.starts_with("ACRE_B64_FILE = [\n    \""));
        assert!(text.ends_with("    \"\"\n];"));
        let joined: String = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix('"')?.strip_suffix("\","))
            .collect();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(joined)
                .unwrap(),
            w
        );
    }

    #[test]
    fn matches_the_shipped_sounds() {
        // The repo's .b64 files were made by the C++ tool: decode one and re-encode it.
        let path = format!(
            "{}/../../../addons/sys_sounds/sounds/beep.b64",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let joined: String = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix('"')?.strip_suffix("\","))
            .collect();
        let wav = base64::engine::general_purpose::STANDARD
            .decode(joined)
            .unwrap();
        assert_eq!(encode(&wav), text.trim_end());
    }
}

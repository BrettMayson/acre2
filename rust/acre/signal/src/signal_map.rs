//! The signal map debug tool (`acre_sys_signalmap`): multipath signal from one transmitter over
//! a grid of receiver positions, drawn as a texture the map overlay shows.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::{
    antenna::Antenna,
    map::Map,
    math::Vec3,
    models::{Link, Signal, multipath::Multipath},
};

/// The overlay tile always covers 4096 m.
pub const TILE_METRES: u32 = 4096;
/// One ARGB8888 mipmap must stay under 16 MiB.
const MAX_TEXTURE: u32 = 1024;

#[derive(Debug, Clone)]
pub struct Area {
    pub start: Vec3,
    pub end: Vec3,
    pub sample_size: f32,
    pub tx_pos: Vec3,
    pub tx_dir: Vec3,
    pub tx_height: f32,
    pub rx_height: f32,
    pub frequency: f32,
    pub power: f32,
    pub lower_sensitivity: f32,
    pub upper_sensitivity: f32,
    pub omnidirectional: bool,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub tx_pos: Vec3,
    pub rx_pos: Vec3,
    pub signal: Signal,
}

#[derive(Debug, Clone)]
pub struct Result {
    pub count_x: u32,
    pub count_y: u32,
    /// Row by row (`y * count_x + x`).
    pub samples: Vec<Sample>,
}

impl Result {
    #[must_use]
    pub fn sample(&self, x: u32, y: u32) -> Option<&Sample> {
        if x >= self.count_x || y >= self.count_y {
            return None;
        }
        self.samples.get((y * self.count_x + x) as usize)
    }
}

/// Computes the area. `progress` counts finished samples (out of `count_x × count_y`, stored in
/// `total` first).
#[must_use]
pub fn compute(
    map: &Map,
    multipath: &Multipath,
    tx_antenna: &Antenna,
    rx_antenna: &Antenna,
    area: &Area,
    progress: &AtomicU32,
    total: &AtomicU32,
) -> Result {
    let sample = area.sample_size.floor().max(1.0);
    let extent = map.cell_size() * map.size() as f32;
    let mut width = area.end.x - area.start.x;
    let mut height = area.end.y - area.start.y;
    // As in the C++ (which grew the area here rather than shrinking it).
    if area.start.x + width > extent {
        width -= extent - (area.start.x + width);
    }
    if area.start.y + height > extent {
        height -= extent - (area.start.y + height);
    }
    let count_x = (width / sample).floor().max(0.0) as u32;
    let count_y = (height / sample).floor().max(0.0) as u32;
    total.store(count_x * count_y, Ordering::Relaxed);
    progress.store(0, Ordering::Relaxed);
    let mut tx_pos = area.tx_pos;
    tx_pos.z += area.tx_height;

    let rows: Vec<u32> = (0..count_y).collect();
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunk = rows.len().div_ceil(threads).max(1);
    let samples: Vec<Sample> = std::thread::scope(|s| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|ys| {
                s.spawn(move || {
                    let mut out = Vec::with_capacity(ys.len() * count_x as usize);
                    for &y in ys {
                        for x in 0..count_x {
                            let rx_x = area.start.x + x as f32 * sample + sample / 2.0;
                            let rx_y = area.start.y + y as f32 * sample + sample / 2.0;
                            let rx_pos =
                                Vec3::new(rx_x, rx_y, map.elevation(rx_x, rx_y) + area.rx_height);
                            let link = Link {
                                tx_pos,
                                tx_dir: area.tx_dir,
                                rx_pos,
                                rx_dir: Vec3::new(0.0, 1.0, 0.0),
                                tx_antenna,
                                rx_antenna,
                                frequency: area.frequency,
                                power: area.power,
                                scale: 1.0,
                                omnidirectional: area.omnidirectional,
                            };
                            out.push(Sample {
                                tx_pos,
                                rx_pos,
                                signal: multipath.process(&link),
                            });
                        }
                        progress.fetch_add(count_x, Ordering::Relaxed);
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("signal map thread"))
            .collect()
    });
    Result {
        count_x,
        count_y,
        samples,
    }
}

/// The area as an uncompressed ARGB8888 PAA covering [`TILE_METRES`] from the area's start:
/// green and opaque where the signal is strong, red and clear where it's weak.
#[must_use]
pub fn texture(result: &Result, area: &Area) -> Vec<u8> {
    let sample = area.sample_size.floor().max(1.0);
    // Metres per pixel: a power of two, at least what fits the size limit.
    let mut m_per_px = (TILE_METRES / MAX_TEXTURE).max(1);
    while m_per_px * 2 <= sample as u32 && m_per_px * 2 <= TILE_METRES {
        m_per_px *= 2;
    }
    let size = TILE_METRES / m_per_px;
    let span = (area.upper_sensitivity - area.lower_sensitivity).max(f32::EPSILON);
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for py in 0..size {
        for px in 0..size {
            let sx = ((px * m_per_px) as f32 / sample) as u32;
            let sy = ((py * m_per_px) as f32 / sample) as u32;
            let Some(s) = result.sample(sx, sy) else {
                continue;
            };
            let p = ((s.signal.dbm - area.lower_sensitivity) / span).clamp(0.0, 1.0);
            // Rows go top (north) to bottom.
            let i = (((size - 1 - py) * size + px) * 4) as usize;
            pixels[i] = 0; // B
            pixels[i + 1] = (255.0 * p) as u8; // G
            pixels[i + 2] = (255.0 * (1.0 - p)) as u8; // R
            pixels[i + 3] = (225.0 * p) as u8; // A
        }
    }
    paa_argb8888(size, &pixels)
}

/// A one-mipmap ARGB8888 PAA from BGRA pixels.
fn paa_argb8888(size: u32, bgra: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bgra.len() + 128);
    out.extend_from_slice(&[0x88, 0x88]);
    let tagg = |out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(b"GGAT");
        out.extend_from_slice(name);
        out.extend_from_slice(&u32::try_from(data.len()).expect("small").to_le_bytes());
        out.extend_from_slice(data);
    };
    tagg(&mut out, b"GALF", &[1, 0, 0, 0]); // has alpha
    // Offsets of the mipmaps: after this tagg (12 + 64 bytes) and the palette (2).
    let mip_offset = u32::try_from(out.len() + 12 + 64 + 2).expect("small");
    let mut offsets = [0u8; 64];
    offsets[..4].copy_from_slice(&mip_offset.to_le_bytes());
    tagg(&mut out, b"SFFO", &offsets);
    out.extend_from_slice(&0u16.to_le_bytes()); // no palette
    let edge = u16::try_from(size).expect("texture size");
    out.extend_from_slice(&edge.to_le_bytes());
    out.extend_from_slice(&edge.to_le_bytes());
    let len = u32::try_from(bgra.len()).expect("mipmap size");
    out.extend_from_slice(&len.to_le_bytes()[..3]);
    out.extend_from_slice(bgra);
    out.extend_from_slice(&[0; 6]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paa_layout() {
        let paa = paa_argb8888(2, &[1; 16]);
        assert_eq!(&paa[..2], &[0x88, 0x88]);
        assert_eq!(&paa[2..10], b"GGATGALF");
        let offset = u32::from_le_bytes(paa[30..34].try_into().unwrap()) as usize;
        assert_eq!(&paa[offset..offset + 4], &[2, 0, 2, 0]);
        assert_eq!(&paa[offset + 4..offset + 7], &[16, 0, 0]);
        assert_eq!(paa.len(), offset + 7 + 16 + 6);
    }
}

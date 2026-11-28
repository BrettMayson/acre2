//! The two decompressors Arma's files use: LZO1X (WRP v23+) and LZSS (older WRPs and
//! compressed PBO entries).

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecompressError {
    #[error("compressed data ended early")]
    InputOverrun,
    #[error("decompressed data is larger than expected")]
    OutputOverrun,
    #[error("back-reference points before the start of the output")]
    LookBehindOverrun,
    #[error("stream ended before the expected size ({got} of {expected} bytes)")]
    OutputUnderrun { got: usize, expected: usize },
    #[error("checksum mismatch")]
    Checksum,
}

struct Input<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Input<'_> {
    fn next(&mut self) -> Result<usize, DecompressError> {
        let b = *self
            .data
            .get(self.pos)
            .ok_or(DecompressError::InputOverrun)?;
        self.pos += 1;
        Ok(usize::from(b))
    }

    fn peek(&self, offset: usize) -> Result<usize, DecompressError> {
        self.data
            .get(self.pos + offset)
            .map(|b| usize::from(*b))
            .ok_or(DecompressError::InputOverrun)
    }

    /// Counts a run of zero bytes (each worth 255), consuming them.
    fn zero_run(&mut self) -> Result<usize, DecompressError> {
        let mut t = 0;
        while self.peek(0)? == 0 {
            t += 255;
            self.pos += 1;
        }
        Ok(t)
    }
}

struct Output {
    data: Vec<u8>,
    limit: usize,
}

impl Output {
    fn literals(&mut self, input: &mut Input, n: usize) -> Result<(), DecompressError> {
        if self.data.len() + n > self.limit {
            return Err(DecompressError::OutputOverrun);
        }
        let src = input
            .data
            .get(input.pos..input.pos + n)
            .ok_or(DecompressError::InputOverrun)?;
        self.data.extend_from_slice(src);
        input.pos += n;
        Ok(())
    }

    /// Copies `n` bytes from `distance` bytes back (they may overlap what is being written).
    fn back_copy(&mut self, distance: usize, n: usize) -> Result<(), DecompressError> {
        if distance == 0 || distance > self.data.len() {
            return Err(DecompressError::LookBehindOverrun);
        }
        if self.data.len() + n > self.limit {
            return Err(DecompressError::OutputOverrun);
        }
        let start = self.data.len() - distance;
        for i in 0..n {
            let b = self.data[start + i];
            self.data.push(b);
        }
        Ok(())
    }
}

enum State {
    Top,
    FirstLiteralRun,
    Match(usize),
    MatchDone,
    MatchNext(usize),
}

/// LZO1X decompression of exactly `out_len` bytes, as Mikero's variant used by the C++: the
/// input length isn't known up front, so it returns how many input bytes it consumed.
pub fn lzo1x(src: &[u8], out_len: usize) -> Result<(Vec<u8>, usize), DecompressError> {
    let mut input = Input { data: src, pos: 0 };
    let mut out = Output {
        data: Vec::with_capacity(out_len),
        limit: out_len,
    };

    let mut state = State::Top;
    if input.peek(0)? > 17 {
        let t = input.next()? - 17;
        if t < 4 {
            state = State::MatchNext(t);
        } else {
            out.literals(&mut input, t)?;
            state = State::FirstLiteralRun;
        }
    }

    loop {
        state = match state {
            State::Top => {
                let mut t = input.next()?;
                if t >= 16 {
                    State::Match(t)
                } else {
                    if t == 0 {
                        t += input.zero_run()?;
                        t += 15 + input.next()?;
                    }
                    out.literals(&mut input, t + 3)?;
                    State::FirstLiteralRun
                }
            }
            State::FirstLiteralRun => {
                let t = input.next()?;
                if t >= 16 {
                    State::Match(t)
                } else {
                    let distance = 1 + 0x0800 + (t >> 2) + (input.next()? << 2);
                    out.back_copy(distance, 3)?;
                    State::MatchDone
                }
            }
            State::Match(mut t) => {
                if t >= 64 {
                    let distance = 1 + ((t >> 2) & 7) + (input.next()? << 3);
                    out.back_copy(distance, (t >> 5) + 1)?;
                } else if t >= 32 {
                    t &= 31;
                    if t == 0 {
                        t += input.zero_run()?;
                        t += 31 + input.next()?;
                    }
                    let distance = 1 + (input.peek(0)? >> 2) + (input.peek(1)? << 6);
                    input.pos += 2;
                    out.back_copy(distance, t + 2)?;
                } else if t >= 16 {
                    let high = (t & 8) << 11;
                    t &= 7;
                    if t == 0 {
                        t += input.zero_run()?;
                        t += 7 + input.next()?;
                    }
                    let low = (input.peek(0)? >> 2) + (input.peek(1)? << 6);
                    input.pos += 2;
                    if high + low == 0 {
                        // End of stream.
                        if out.data.len() != out_len {
                            return Err(DecompressError::OutputUnderrun {
                                got: out.data.len(),
                                expected: out_len,
                            });
                        }
                        return Ok((out.data, input.pos));
                    }
                    out.back_copy(high + low + 0x4000, t + 2)?;
                } else {
                    let distance = 1 + (t >> 2) + (input.next()? << 2);
                    out.back_copy(distance, 2)?;
                }
                State::MatchDone
            }
            State::MatchDone => {
                let t = *input
                    .data
                    .get(input.pos.wrapping_sub(2))
                    .ok_or(DecompressError::InputOverrun)? as usize
                    & 3;
                if t == 0 {
                    State::Top
                } else {
                    State::MatchNext(t)
                }
            }
            State::MatchNext(t) => {
                out.literals(&mut input, t)?;
                State::Match(input.next()?)
            }
        };
    }
}

/// LZSS decompression of exactly `out_len` bytes, followed by the 4-byte checksum. Returns the
/// data and how many input bytes it consumed.
pub fn lzss(src: &[u8], out_len: usize) -> Result<(Vec<u8>, usize), DecompressError> {
    const N: usize = 4096;
    const F: usize = 18;
    const THRESHOLD: usize = 2;

    let mut input = Input { data: src, pos: 0 };
    let mut out = Vec::with_capacity(out_len);
    let mut text = [b' '; N];
    let mut r = N - F;
    let mut flags: u32 = 0;
    let mut checksum: u32 = 0;
    let mut remaining = out_len;

    while remaining > 0 {
        flags >>= 1;
        if flags & 256 == 0 {
            flags = input.next()? as u32 | 0xff00;
        }
        if flags & 1 != 0 {
            let c = input.next()? as u8;
            checksum = checksum.wrapping_add(u32::from(c));
            out.push(c);
            remaining -= 1;
            text[r] = c;
            r = (r + 1) & (N - 1);
        } else {
            let mut i = input.next()?;
            let mut j = input.next()?;
            i |= (j & 0xf0) << 4;
            j = (j & 0x0f) + THRESHOLD;
            if j + 1 > remaining {
                return Err(DecompressError::OutputOverrun);
            }
            let start = r.wrapping_sub(i);
            for k in 0..=j {
                let c = text[start.wrapping_add(k) & (N - 1)];
                checksum = checksum.wrapping_add(u32::from(c));
                out.push(c);
                remaining -= 1;
                text[r] = c;
                r = (r + 1) & (N - 1);
            }
        }
    }

    let stored = src
        .get(input.pos..input.pos + 4)
        .ok_or(DecompressError::InputOverrun)?;
    if u32::from_le_bytes(stored.try_into().expect("4 bytes")) != checksum {
        return Err(DecompressError::Checksum);
    }
    Ok((out, input.pos + 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A literal-only LZO1X stream: one run, then the end marker.
    fn lzo_literals(data: &[u8]) -> Vec<u8> {
        assert!(data.len() > 3 && data.len() <= 238);
        let mut v = vec![u8::try_from(data.len() + 17).unwrap()];
        v.extend_from_slice(data);
        v.extend_from_slice(&[0x11, 0, 0]);
        v
    }

    #[test]
    fn lzo_literal_run() {
        let data: Vec<u8> = (0..200u8).collect();
        let mut stream = lzo_literals(&data);
        stream.extend_from_slice(&[0xAA; 16]); // trailing bytes that aren't ours
        let (out, used) = lzo1x(&stream, data.len()).unwrap();
        assert_eq!(out, data);
        assert_eq!(used, data.len() + 4);
    }

    #[test]
    fn lzo_wrong_size() {
        let data: Vec<u8> = (0..100u8).collect();
        let stream = lzo_literals(&data);
        assert!(lzo1x(&stream, 99).is_err());
        assert!(lzo1x(&stream, 101).is_err());
        assert!(lzo1x(&stream[..50], 100).is_err());
    }

    #[test]
    fn lzss_literals_and_backref() {
        // flags 0b1111_1110... : first byte a back-reference? Build: 4 literals then a match.
        // Flags: bits 0..3 literals, bit 4 a back-reference.
        let mut stream = vec![0b0000_1111, b'a', b'b', b'c', b'd'];
        // Back-reference 4 bytes back (BI's LZSS stores a distance), length 1 + 2 + 1 = 4.
        stream.extend_from_slice(&[4, 1]);
        let expected = b"abcdabcd";
        let sum: u32 = expected.iter().map(|&b| u32::from(b)).sum();
        stream.extend_from_slice(&sum.to_le_bytes());
        let (out, used) = lzss(&stream, 8).unwrap();
        assert_eq!(out, expected);
        assert_eq!(used, stream.len());
    }
}

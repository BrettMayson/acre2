//! Terrain files: Arma's binarised WRP (`OPRW`, versions 17+), the old `8WVR`, and ACRE's own
//! `.fakewrp` (header `ACRE`) for terrains whose WRP can't be read. Only the elevation grid and
//! the peaks are kept.
//!
//! <https://community.bistudio.com/wiki/Wrp_File_Format_-_OPRWv17_to_24>

use thiserror::Error;

use crate::compress::{self, DecompressError};

#[derive(Debug, Error)]
pub enum WrpError {
    #[error("file ended early")]
    Truncated,
    #[error("unknown terrain file type {0:?}")]
    UnknownType([u8; 4]),
    #[error(transparent)]
    Decompress(#[from] DecompressError),
}

/// How the load went: the C++ returned 0 / -1 / -2, which SQF still checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadResult {
    Success = 0,
    /// The header was read but not the elevations: the terrain is treated as flat (0 m).
    Recovered = -1,
    Failure = -2,
}

#[derive(Debug, Clone)]
pub struct Landscape {
    pub filetype: [u8; 4],
    pub version: u32,
    pub layer_size: (u32, u32),
    pub map_size: (u32, u32),
    /// Layer cell size as stored.
    pub cell_size: f32,
    /// Distance between elevation samples, in metres.
    pub grid_size: f32,
    /// `map_size.0 × map_size.1` samples, row by row (y major).
    pub elevations: Vec<f32>,
    /// `[x, y, z]` as stored (`z` is the horizontal axis, `y` the height).
    pub peaks: Vec<[f32; 3]>,
    pub result: LoadResult,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], WrpError> {
        let b = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or(WrpError::Truncated)?;
        self.pos += n;
        Ok(b)
    }

    fn u8(&mut self) -> Result<u8, WrpError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, WrpError> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().expect("2")))
    }

    fn u32(&mut self) -> Result<u32, WrpError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().expect("4")))
    }

    fn f32(&mut self) -> Result<f32, WrpError> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().expect("4")))
    }

    fn rest(&self) -> &'a [u8] {
        &self.data[self.pos.min(self.data.len())..]
    }

    /// A grid block (quad tree) is only skipped, exactly as the C++ walked it: including where
    /// it gives up early on a node outside the grid.
    fn skip_binary_tree_block(
        &mut self,
        bit_length: u32,
        block_size: u32,
        cell_size: u32,
        offset_x: u32,
        offset_y: u32,
    ) -> Result<bool, WrpError> {
        let mut size = 1u32;
        if block_size != 0 {
            let exp = ((f64::from(block_size).log10() / 2f64.log10()) / 2.0).ceil();
            size = 2f64.powf(2.0 * exp) as u32;
        }
        let child_size = size / 4;
        let mut flags = self.u16()?;
        for bit in 0..16u32 {
            let x = offset_x + (bit % 4) * size * bit_length;
            let y = offset_y + (bit / 4) * size;
            if flags & 1 != 0 {
                if !self.skip_binary_tree_block(bit_length, child_size, cell_size, x, y)? {
                    return Ok(false);
                }
            } else {
                let a = self.bytes(bit_length as usize)?;
                let b = self.bytes(bit_length as usize)?;
                let filler = a.iter().chain(b).all(|v| *v == 0);
                if !filler && (x >= cell_size || y >= cell_size) {
                    return Ok(false);
                }
            }
            flags >>= 1;
        }
        Ok(true)
    }

    fn grid_block(&mut self, cell_size: u32) -> Result<(), WrpError> {
        if self.u8()? == 1 {
            self.skip_binary_tree_block(2, 16, cell_size, 0, 0)?;
        } else {
            self.u32()?; // fill value
        }
        Ok(())
    }

    /// An array of `count` values of `size` bytes, compressed when it's 1024 bytes or more.
    fn compressed(&mut self, count: usize, size: usize, version: u32) -> Result<Vec<u8>, WrpError> {
        let len = count * size;
        if len < 1024 {
            return Ok(self.bytes(len)?.to_vec());
        }
        let (data, used) = if version >= 23 {
            compress::lzo1x(self.rest(), len)?
        } else {
            compress::lzss(self.rest(), len)?
        };
        self.pos += used;
        Ok(data)
    }
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

impl Landscape {
    /// Parses a terrain file. A file whose header is readable but not its elevations still
    /// loads, flat, with [`LoadResult::Recovered`] (as the C++ did).
    pub fn parse(data: &[u8]) -> Result<Self, WrpError> {
        let mut r = Reader { data, pos: 0 };
        let filetype: [u8; 4] = r.bytes(4)?.try_into().expect("4");

        if &filetype == b"8WVR" {
            let layer_size = (r.u32()?, r.u32()?);
            let map_size = (r.u32()?, r.u32()?);
            let cell_size = r.f32()?;
            let count = map_size.0 as usize * map_size.0 as usize;
            let elevations = floats(r.bytes(count * 4)?);
            return Ok(Self {
                filetype,
                version: 0,
                layer_size,
                map_size,
                cell_size,
                grid_size: cell_size,
                elevations,
                peaks: Vec::new(),
                result: LoadResult::Success,
            });
        }
        if &filetype != b"OPRW" && &filetype != b"ACRE" {
            return Err(WrpError::UnknownType(filetype));
        }

        let version = r.u32()?;
        if version > 24 {
            r.u32()?;
        }
        let layer_size = (r.u32()?, r.u32()?);
        let map_size = (r.u32()?, r.u32()?);
        let cell_size = r.f32()?;
        let grid_size = cell_size * layer_size.0 as f32 / map_size.0 as f32;
        let mut landscape = Self {
            filetype,
            version,
            layer_size,
            map_size,
            cell_size,
            grid_size,
            elevations: Vec::new(),
            peaks: Vec::new(),
            result: LoadResult::Success,
        };

        let body = |r: &mut Reader, l: &mut Self| -> Result<(), WrpError> {
            r.grid_block(map_size.0)?; // CellEnv
            r.grid_block(map_size.0)?; // CfgEnvSounds
            let peak_count = r.u32()? as usize;
            let peaks = floats(r.bytes(peak_count * 12)?);
            l.peaks = peaks.as_chunks::<3>().0.to_vec();
            r.grid_block(layer_size.0)?; // RvmatLayerIndex
            let count = map_size.0 as usize * map_size.1 as usize;
            if &filetype == b"ACRE" {
                // Written uncompressed, and square.
                let square = map_size.0 as usize * map_size.0 as usize;
                l.elevations = floats(r.bytes(square * 4)?);
            } else {
                // Random clutter, then (v22+) another byte grid, then the elevations.
                if version <= 20 {
                    r.compressed(layer_size.0 as usize * layer_size.1 as usize, 2, version)?;
                } else {
                    r.compressed(count, 1, version)?;
                }
                if version >= 22 {
                    r.compressed(count, 1, version)?;
                }
                l.elevations = floats(&r.compressed(count, 4, version)?);
            }
            Ok(())
        };

        if let Err(e) = body(&mut r, &mut landscape) {
            tracing::error!(error = %e, "WRP elevations unreadable, treating the terrain as flat");
            landscape.peaks = vec![[
                map_size.0 as f32 * grid_size / 2.0,
                map_size.1 as f32 * grid_size / 2.0,
                0.0,
            ]];
            landscape.elevations = vec![0.0; map_size.0 as usize * map_size.1 as usize];
            landscape.result = LoadResult::Recovered;
        }
        Ok(landscape)
    }

    /// Writes ACRE's `.fakewrp` format (what the compat addons ship for terrains whose WRP
    /// can't be read).
    #[must_use]
    pub fn to_fakewrp(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.peaks.len() * 12 + self.elevations.len() * 4);
        let u32le = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
        out.extend_from_slice(b"ACRE");
        u32le(&mut out, self.version.max(25));
        u32le(&mut out, 0);
        u32le(&mut out, self.layer_size.0);
        u32le(&mut out, self.layer_size.1);
        u32le(&mut out, self.map_size.0);
        u32le(&mut out, self.map_size.1);
        out.extend_from_slice(&self.cell_size.to_le_bytes());
        for _ in 0..2 {
            out.push(0);
            u32le(&mut out, 0);
        }
        u32le(
            &mut out,
            u32::try_from(self.peaks.len()).expect("peak count"),
        );
        for p in &self.peaks {
            for v in p {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out.push(0);
        u32le(
            &mut out,
            u32::try_from(self.peaks.len()).expect("peak count"),
        );
        for e in &self.elevations {
            out.extend_from_slice(&e.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn landscape(size: u32) -> Landscape {
        Landscape {
            filetype: *b"ACRE",
            version: 25,
            layer_size: (size / 2, size / 2),
            map_size: (size, size),
            cell_size: 20.0,
            grid_size: 10.0,
            elevations: (0..size * size).map(|i| i as f32 * 0.5).collect(),
            peaks: vec![[100.0, 50.0, 200.0], [10.0, 7.0, 20.0]],
            result: LoadResult::Success,
        }
    }

    #[test]
    fn fakewrp_roundtrip() {
        let l = landscape(64);
        let parsed = Landscape::parse(&l.to_fakewrp()).unwrap();
        assert_eq!(parsed.result, LoadResult::Success);
        assert_eq!(parsed.map_size, (64, 64));
        assert!((parsed.grid_size - 10.0).abs() < 1e-6);
        assert_eq!(parsed.peaks, l.peaks);
        assert_eq!(parsed.elevations, l.elevations);
    }

    #[test]
    fn truncated_elevations_recover_flat() {
        let l = landscape(64);
        let bytes = l.to_fakewrp();
        let parsed = Landscape::parse(&bytes[..bytes.len() - 100]).unwrap();
        assert_eq!(parsed.result, LoadResult::Recovered);
        assert_eq!(parsed.elevations.len(), 64 * 64);
        assert!(parsed.elevations.iter().all(|e| *e == 0.0));
        assert_eq!(parsed.peaks.len(), 1);
    }

    #[test]
    fn garbage_fails() {
        assert!(Landscape::parse(b"NOPE").is_err());
        assert!(Landscape::parse(b"OP").is_err());
    }

    /// An `OPRW` v25 with LZO-compressed (literal-only) grids.
    #[test]
    fn oprw_with_lzo() {
        let size = 16u32; // 256 samples: the byte grids are < 1024 B, the elevations aren't
        let count = (size * size) as usize;
        let elevations: Vec<f32> = (0..count).map(|i| i as f32).collect();
        let mut out = Vec::new();
        out.extend_from_slice(b"OPRW");
        for v in [25u32, 0, size, size, size, size] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&10f32.to_le_bytes());
        for _ in 0..2 {
            out.push(0);
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        out.extend_from_slice(&1u32.to_le_bytes());
        for v in [5f32, 6.0, 7.0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(0);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&vec![1u8; count]); // clutter, uncompressed (< 1024 B)
        out.extend_from_slice(&vec![2u8; count]);
        // Elevations: 1024 bytes as several literal runs is not valid LZO, so as one long run
        // (t == 0 form): 0, zero bytes worth 255 each, remainder.
        let raw: Vec<u8> = elevations.iter().flat_map(|e| e.to_le_bytes()).collect();
        let run = raw.len() - 3; // literals = t + 3 with t = 15 + extra
        let mut extra = run - 15;
        out.push(0);
        while extra > 255 {
            out.push(0);
            extra -= 255;
        }
        out.push(u8::try_from(extra).unwrap());
        out.extend_from_slice(&raw);
        out.extend_from_slice(&[0x11, 0, 0]);
        let parsed = Landscape::parse(&out).unwrap();
        assert_eq!(parsed.result, LoadResult::Success, "{parsed:?}");
        assert_eq!(parsed.elevations, elevations);
        assert_eq!(parsed.peaks, vec![[5.0, 6.0, 7.0]]);
    }
}

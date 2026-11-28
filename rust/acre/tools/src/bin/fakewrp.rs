//! Makes a `.fakewrp` (YARD/ACRE terrain file) for terrains whose WRP the extension can't read
//! (encrypted or unusual). Replaces the C++ `fakewrpgen`.
//!
//! - `fakewrp convert <input.wrp> <output.fakewrp>`: from a readable WRP.
//! - `fakewrp generate --world-size <m> --map-size <n> <heights.csv> <output.fakewrp>`: from
//!   heights dumped in game. The CSV is `map_size × map_size` comma-separated heights, row by
//!   row from the south-west corner, e.g. from:
//!
//! ```sqf
//! private _mapSize = worldSize;
//! private _gridSize = 4096;
//! private _interval = _mapSize / _gridSize;
//! private _heights = [];
//! for "_y" from 0 to (_mapSize - _interval) step _interval do {
//!     for "_x" from 0 to (_mapSize - _interval) step _interval do {
//!         _heights pushBack getTerrainHeightASL [_x, _y];
//!     };
//! };
//! copyToClipboard (_heights joinString ",");
//! ```
//!
//! Then reference it from a `CfgAcreWorlds` entry (`wrp = "\path\to\terrain.fakewrp";`).

use std::path::PathBuf;

use acre2_signal::{
    map::Map,
    wrp::{Landscape, LoadResult},
};
use anyhow::{Context, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Makes a .fakewrp terrain file")]
struct Args {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Convert a readable WRP.
    Convert { input: PathBuf, output: PathBuf },
    /// Build from a CSV of heights.
    Generate {
        /// Terrain size in metres (`worldSize`).
        #[arg(long)]
        world_size: f32,
        /// Samples per side in the CSV.
        #[arg(long, default_value_t = 4096)]
        map_size: u32,
        input: PathBuf,
        output: PathBuf,
    },
}

/// Peaks found from the grid, in the file's `[x, height, y]` order.
fn grid_peaks(map_size: u32, grid_size: f32, elevations: &[f32]) -> Vec<[f32; 3]> {
    let mut map = Map::from_grid(map_size, grid_size, elevations.to_vec());
    map.generate_peaks_apart(30.0);
    map.peaks.iter().map(|p| [p.x, p.z, p.y]).collect()
}

fn generate(world_size: f32, map_size: u32, csv: &str) -> anyhow::Result<Landscape> {
    let elevations = csv
        .split(',')
        .map(|v| v.trim().trim_matches('\0'))
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.parse::<f32>()
                .with_context(|| format!("not a height: {v:?}"))
        })
        .collect::<anyhow::Result<Vec<f32>>>()?;
    let expected = map_size as usize * map_size as usize;
    if elevations.len() != expected {
        bail!(
            "expected {expected} heights ({map_size} × {map_size}), found {}",
            elevations.len()
        );
    }
    let grid_size = world_size / map_size as f32;
    Ok(Landscape {
        filetype: *b"ACRE",
        version: 25,
        layer_size: (map_size, map_size),
        map_size: (map_size, map_size),
        cell_size: grid_size,
        grid_size,
        peaks: grid_peaks(map_size, grid_size, &elevations),
        elevations,
        result: LoadResult::Success,
    })
}

fn main() -> anyhow::Result<()> {
    let landscape = match Args::parse().task {
        Task::Convert { input, output } => {
            let data =
                std::fs::read(&input).with_context(|| format!("reading {}", input.display()))?;
            let mut l = Landscape::parse(&data)?;
            if l.result != LoadResult::Success {
                bail!("the WRP's elevations couldn't be read");
            }
            if &l.filetype == b"8WVR" {
                l.peaks = grid_peaks(l.map_size.0, l.grid_size, &l.elevations);
            }
            (l, output)
        }
        Task::Generate {
            world_size,
            map_size,
            input,
            output,
        } => {
            let csv = std::fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            (generate(world_size, map_size, &csv)?, output)
        }
    };
    let (landscape, output) = landscape;
    std::fs::write(&output, landscape.to_fakewrp())
        .with_context(|| format!("writing {}", output.display()))?;
    println!(
        "{}: {} × {} samples, {} m apart, {} peaks",
        output.display(),
        landscape.map_size.0,
        landscape.map_size.1,
        landscape.grid_size,
        landscape.peaks.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_roundtrips() {
        let size = 64u32;
        let heights: Vec<String> = (0..size * size)
            .map(|i| format!("{}", 10.0 + ((i % size) as f32 * 0.3).sin() * 20.0))
            .collect();
        let l = generate(640.0, size, &heights.join(",")).unwrap();
        let parsed = Landscape::parse(&l.to_fakewrp()).unwrap();
        assert_eq!(parsed.result, LoadResult::Success);
        assert!((parsed.grid_size - 10.0).abs() < 1e-6);
        assert_eq!(parsed.elevations, l.elevations);
        // Peaks are stored [x, height, y]: the loaded map sees them at the right height.
        let map = Map::from_landscape(&parsed);
        for p in &map.peaks {
            assert!((map.elevation(p.x, p.y) - p.z).abs() < 1e-3, "{p:?}");
        }
        assert!(generate(640.0, size, "1,2,3").is_err());
    }
}

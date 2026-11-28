//! The terrain as the propagation models see it: a square elevation grid, plus peaks
//! (reflection candidates for the multipath model).

use crate::{
    math::Vec3,
    wrp::{Landscape, LoadResult},
};

/// ITM radio climate, as SQF passes it (`acre_sys_signal_fnc_getRadioClimateCode`, 1–7).
pub type Climate = i32;

#[derive(Debug, Clone)]
pub struct Map {
    size: u32,
    cell_size: f32,
    inv_cell_size: f32,
    elevations: Vec<f32>,
    pub peaks: Vec<Vec3>,
    pub climate: Climate,
}

impl Map {
    /// A map from an elevation grid (`size × size`, row by row).
    #[must_use]
    pub fn from_grid(size: u32, cell_size: f32, mut elevations: Vec<f32>) -> Self {
        elevations.resize(size as usize * size as usize, 0.0);
        Self {
            size,
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            elevations,
            peaks: Vec::new(),
            climate: 5,
        }
    }

    /// The map for a parsed terrain file. The WRP's own peaks are filtered as the C++ did
    /// (higher than 5 m, and no equal or higher peak within 150 m); `8WVR` files have none, so
    /// they're found from the grid.
    #[must_use]
    pub fn from_landscape(l: &Landscape) -> Self {
        let mut map = Self::from_grid(l.map_size.0, l.grid_size, l.elevations.clone());
        if &l.filetype == b"8WVR" {
            map.generate_peaks();
            return map;
        }
        // Stored as [x, height, y].
        let initial: Vec<Vec3> = l
            .peaks
            .iter()
            .map(|p| Vec3::new(p[0], p[2], p[1]))
            .collect();
        map.peaks = initial
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                p.z > 5.0
                    && !initial
                        .iter()
                        .enumerate()
                        .any(|(c, q)| c != *i && p.distance(*q) <= 150.0 && q.z >= p.z)
            })
            .map(|(_, p)| *p)
            .collect();
        if l.result == LoadResult::Recovered {
            map.peaks.clear();
        }
        map
    }

    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    #[must_use]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Grid sample, clamped to the edges.
    #[must_use]
    pub fn elevation_raw(&self, x: i32, y: i32) -> f32 {
        let max = self.size as i32 - 1;
        let x = x.min(max).max(0) as usize;
        let y = y.min(max).max(0) as usize;
        self.elevations[x + y * self.size as usize]
    }

    /// Height at a world position (triangulated like Arma's terrain), and the slope.
    #[must_use]
    pub fn elevation_slope(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let x_rel = x * self.inv_cell_size;
        let y_rel = y * self.inv_cell_size;
        let xi = x_rel.floor() as i32;
        let yi = y_rel.floor() as i32;
        let x_in = x_rel - xi as f32;
        let y_in = y_rel - yi as f32;
        let z00 = self.elevation_raw(xi, yi);
        let z10 = self.elevation_raw(xi + 1, yi);
        let z01 = self.elevation_raw(xi, yi + 1);
        let z11 = self.elevation_raw(xi + 1, yi + 1);
        if x_in <= 1.0 - y_in {
            let d1000 = z10 - z00;
            let d0100 = z01 - z00;
            (
                z00 + d0100 * y_in + d1000 * x_in,
                d1000 * self.inv_cell_size,
                d0100 * self.inv_cell_size,
            )
        } else {
            let d1011 = z10 - z11;
            let d0111 = z01 - z11;
            (
                z10 + d0111 - d0111 * x_in - d1011 * y_in,
                d0111 * -self.inv_cell_size,
                d1011 * -self.inv_cell_size,
            )
        }
    }

    #[must_use]
    pub fn elevation(&self, x: f32, y: f32) -> f32 {
        self.elevation_slope(x, y).0
    }

    /// The terrain normal at a world position.
    #[must_use]
    pub fn normal(&self, x: f32, y: f32) -> Vec3 {
        let (_, dx, dy) = self.elevation_slope(x, y);
        Vec3::new(-dx, -dy, 1.0).normalize()
    }

    /// Ground heights every `precision` metres along the line from `start` towards `end` (the
    /// end itself isn't sampled).
    #[must_use]
    pub fn terrain_profile(&self, start: Vec3, end: Vec3, precision: f32) -> Vec<f32> {
        let dir = (end - start).normalize();
        let distance = start.distance(end);
        let steps = (distance / precision).floor() as u32;
        (0..steps)
            .map(|i| {
                let p = dir * (i as f32 * precision);
                self.elevation(p.x + start.x, p.y + start.y)
            })
            .collect()
    }

    fn is_peak(&self, x: i32, y: i32) -> bool {
        let h = self.elevation_raw(x, y);
        [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, -1),
            (0, 1),
        ]
        .iter()
        .all(|(dx, dy)| self.elevation_raw(x + dx, y + dy) < h)
    }

    /// Finds peaks in the grid (for terrains that don't list them), at least 150 m apart.
    pub fn generate_peaks(&mut self) {
        self.generate_peaks_apart(150.0);
    }

    /// Finds peaks at least `filter_distance` metres apart (`fakewrpgen` used 30).
    pub fn generate_peaks_apart(&mut self, filter_distance: f32) {
        let size = self.size as i32;
        let mut first_pass = Vec::new();
        for x in 0..size {
            for y in 0..size {
                if self.is_peak(x, y) && self.elevation_raw(x, y) > 0.0 {
                    first_pass.push((x, y));
                }
            }
        }
        let distance = (filter_distance / self.cell_size).ceil() as i32;
        let mut filtered: Vec<(i32, i32)> = Vec::new();
        for (px, py) in first_pass {
            let peak_height = self.elevation_raw(px, py);
            let (x_min, y_min) = ((px - distance).max(0), (py - distance).max(0));
            let (x_max, y_max) = ((px + distance).min(size - 1), (py + distance).min(size - 1));
            let mut is_max = true;
            let mut average = 0.0f32;
            let mut c = 0i32;
            'outer: for x in x_min..=x_max {
                for y in y_min..=y_max {
                    let h = self.elevation_raw(x, y);
                    average += h;
                    // The C++ only broke out of the inner loop.
                    if h >= peak_height && x != px && y != py {
                        is_max = false;
                        break 'outer;
                    }
                    c += 1;
                }
            }
            if is_max {
                average /= c as f32;
                if average / peak_height > 0.5
                    && !filtered.iter().any(|(fx, fy)| {
                        let d =
                            (((px - fx) as f32).powf(2.0) + ((py - fy) as f32).powf(2.0)).sqrt();
                        d * self.cell_size < filter_distance
                    })
                {
                    filtered.push((px, py));
                }
            }
        }
        self.peaks = filtered
            .into_iter()
            .map(|(x, y)| {
                Vec3::new(
                    x as f32 * self.cell_size,
                    y as f32 * self.cell_size,
                    self.elevation_raw(x, y),
                )
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slope() -> Map {
        // z = x * 1 + y * 2 (per cell), 10 m cells.
        let size = 32;
        let e = (0..size * size)
            .map(|i| (i % size) as f32 + 2.0 * (i / size) as f32)
            .collect();
        Map::from_grid(size, 10.0, e)
    }

    #[test]
    fn interpolates_and_clamps() {
        let m = slope();
        assert!((m.elevation(0.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((m.elevation(15.0, 0.0) - 1.5).abs() < 1e-5);
        assert!((m.elevation(15.0, 25.0) - 6.5).abs() < 1e-4);
        // Off the map: clamped to the edge.
        assert!((m.elevation(-50.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((m.elevation(10_000.0, 0.0) - 31.0).abs() < 1e-4);
        let n = m.normal(55.0, 55.0);
        assert!((n.length() - 1.0).abs() < 1e-5);
        assert!(n.x < 0.0 && n.y < 0.0 && n.z > 0.0);
    }

    #[test]
    fn profile() {
        let m = slope();
        let p = m.terrain_profile(Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0), 7.5);
        assert_eq!(p.len(), 13);
        assert!((p[2] - 1.5).abs() < 1e-4);
    }

    #[test]
    fn single_peak_found() {
        let size = 64u32;
        // A broad hill (the C++ ignores spikes: the area around must average over half the
        // peak's height).
        let mut e = vec![40.0f32; (size * size) as usize];
        e[(20 + 30 * size) as usize] = 50.0;
        let mut m = Map::from_grid(size, 10.0, e);
        m.generate_peaks();
        assert_eq!(m.peaks, vec![Vec3::new(200.0, 300.0, 50.0)]);
    }
}

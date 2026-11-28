//! Antenna gain patterns (`.aba` files: gain by frequency, direction and elevation).

use thiserror::Error;

use crate::math::Vec3;

#[derive(Debug, Error)]
pub enum AntennaError {
    #[error("antenna file ended early")]
    Truncated,
    #[error("antenna file has an empty or invalid grid")]
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarization {
    Horizontal,
    Vertical,
    Circular,
}

#[derive(Debug, Clone)]
pub struct Antenna {
    min_frequency: f32,
    max_frequency: f32,
    frequency_step: f32,
    elevation_step: f32,
    direction_step: f32,
    total_entries: u32,
    width: u32,
    height: u32,
    /// Vertical gain for each (frequency, direction, elevation).
    gain_v: Vec<f32>,
    pub polarization: Polarization,
    pub internal_loss_dbm: f32,
}

impl Antenna {
    pub fn parse(data: &[u8]) -> Result<Self, AntennaError> {
        let f = |i: usize| -> Result<[u8; 4], AntennaError> {
            data.get(i * 4..i * 4 + 4)
                .map(|b| b.try_into().expect("4"))
                .ok_or(AntennaError::Truncated)
        };
        let float = |i| f(i).map(f32::from_le_bytes);
        let int = |i| f(i).map(u32::from_le_bytes);
        let total_entries = int(3)?;
        let width = int(4)?;
        let height = int(5)?;
        let count = width as usize * height as usize * total_entries as usize;
        if count == 0 {
            return Err(AntennaError::Invalid);
        }
        let header = 8 * 4;
        let body = data
            .get(header..header + count * 8)
            .ok_or(AntennaError::Truncated)?;
        // Each entry is {v, h}; only v is used.
        let gain_v = body
            .as_chunks::<8>()
            .0
            .iter()
            .map(|e| f32::from_le_bytes([e[0], e[1], e[2], e[3]]))
            .collect();
        Ok(Self {
            min_frequency: float(0)?,
            max_frequency: float(1)?,
            frequency_step: float(2)?,
            elevation_step: float(6)?,
            direction_step: float(7)?,
            total_entries,
            width,
            height,
            gain_v,
            polarization: Polarization::Vertical,
            internal_loss_dbm: 3.0,
        })
    }

    /// Gain in dB towards `signal_dir` for an antenna pointing along `antenna_dir`.
    #[must_use]
    pub fn gain(&self, antenna_dir: Vec3, signal_dir: Vec3, f: f32) -> f32 {
        const DEG: f32 = 57.295_78;
        if f < self.min_frequency || f + self.frequency_step > self.max_frequency {
            return -1000.0;
        }
        if signal_dir.length() <= 0.0 {
            return 0.0;
        }
        let a = antenna_dir.normalize();
        let elev_antenna = a.z.asin() * DEG;
        let dir_antenna = a.x.atan2(a.y) * DEG;
        let s = signal_dir.normalize();
        let elev_signal = s.z.asin() * DEG;
        let dir_signal = s.x.atan2(s.y) * DEG;
        let mut dir = (dir_antenna + dir_signal) % 360.0;
        let mut elev = elev_antenna + elev_signal;
        if !(-90.0..=90.0).contains(&elev) {
            dir = (dir + 180.0) % 360.0;
            if elev < -90.0 {
                elev = -90.0 + (elev.abs() - 90.0);
            } else {
                elev = 90.0 - (elev - 90.0);
            }
        }
        if dir < 0.0 {
            dir += 360.0;
        }
        elev = 90.0 - elev.abs();
        let lower_gain = self.grid_gain(f, dir, elev);
        let upper_gain = self.grid_gain(f + self.frequency_step, dir, elev);
        let lower_freq = f - f % self.frequency_step;
        let upper_freq = lower_freq + self.frequency_step;
        interp(f, lower_freq, upper_freq, lower_gain, upper_gain)
    }

    fn grid_gain(&self, f: f32, dir: f32, elev: f32) -> f32 {
        let index = |v: f32| v.floor().max(0.0) as u32;
        let f_index =
            index((f - self.min_frequency) / self.frequency_step).min(self.total_entries - 1);
        let dir_min = index(dir / self.direction_step);
        let mut dir_max = index((dir + self.direction_step) / self.direction_step);
        if dir_max > self.height - 1 {
            dir_max = 0;
        }
        let elev_min = index(elev / self.elevation_step);
        let elev_max =
            index((elev + self.elevation_step) / self.elevation_step).min(self.width - 1);
        let at = |d: u32, e: u32| {
            let i = f_index as usize * (self.width as usize * self.height as usize)
                + d as usize * self.width as usize
                + e as usize;
            // The C++ didn't bound these; a valid file never goes past the end.
            self.gain_v.get(i).copied().unwrap_or(0.0)
        };
        let dir_lo = dir - dir % self.direction_step;
        let dir_hi = dir_lo + self.direction_step;
        let elev_lo = elev - elev % self.elevation_step;
        let elev_hi = elev_lo + self.elevation_step;
        let lower = interp(
            dir,
            dir_lo,
            dir_hi,
            at(dir_min, elev_min),
            at(dir_max, elev_min),
        );
        let upper = interp(
            dir,
            dir_lo,
            dir_hi,
            at(dir_min, elev_max),
            at(dir_max, elev_max),
        );
        interp(elev, elev_lo, elev_hi, lower, upper)
    }
}

fn interp(g: f32, g1: f32, g2: f32, d1: f32, d2: f32) -> f32 {
    d1 + ((g - g1).max(0.000_01) / (g2 - g1).max(0.000_01)) * (d2 - d1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Antenna {
        let path = format!(
            "{}/../../../addons/sys_antenna/binary/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        Antenna::parse(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn parses_shipped_antennas() {
        let a = load("prc343_gain.aba");
        let up = Vec3::new(0.0, 0.0, 1.0);
        for dir in [Vec3::new(100.0, 0.0, 0.0), Vec3::new(10.0, 30.0, -40.0)] {
            let g = a.gain(up, dir, 2400.0);
            assert!(g.is_finite() && g < 20.0, "{g}");
        }
        assert!((a.gain(up, Vec3::new(1.0, 0.0, 0.0), 10.0) - -1000.0).abs() < f32::EPSILON);
        assert!(a.gain(up, Vec3::default(), 2400.0).abs() < f32::EPSILON);
        assert!(Antenna::parse(&[0; 10]).is_err());
    }
}

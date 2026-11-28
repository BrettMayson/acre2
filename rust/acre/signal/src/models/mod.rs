//! Radio propagation models, ported from the C++ (`extensions/src/ACRE2Arma/signal/models`).

pub mod arcade;
pub mod itm;
pub mod los;
pub mod multipath;

use crate::math::Vec3;

/// One reflection the multipath model used (for the signal map debug tool).
#[derive(Debug, Clone, Copy, Default)]
pub struct Reflection {
    pub point: Vec3,
    pub normal: Vec3,
    pub phase: f32,
    pub reflect_coefficient: f32,
    pub path_budget_dbm: f32,
    pub path_budget_v: f32,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub dbm: f32,
    pub v: f32,
    pub reflections: Vec<Reflection>,
}

impl Default for Signal {
    fn default() -> Self {
        Self {
            dbm: -1000.0,
            v: 0.0,
            reflections: Vec::new(),
        }
    }
}

impl Signal {
    #[must_use]
    pub fn from_dbm(dbm: f32) -> Self {
        Self {
            dbm,
            v: dbm_to_v(dbm, 50.0),
            reflections: Vec::new(),
        }
    }
}

#[must_use]
pub fn dbm_to_v(dbm: f32, r: f32) -> f32 {
    ((r * 0.001) * 10.0f32.powf(dbm * 0.1)).sqrt()
}

/// The C++ divided by 2.3025 (an approximate ln 10), kept for identical results.
#[allow(clippy::approx_constant)]
#[must_use]
pub fn v_to_dbm(v: f32, r: f32) -> f32 {
    10.0 * ((v * v) / (r * 0.001)).ln() / 2.3025
}

#[must_use]
pub fn mw_to_dbm(power_mw: f32) -> f32 {
    10.0 * (power_mw / 1000.0).log10() + 30.0
}

/// Free-space path loss in dB, `frequency` in MHz, `distance` in metres.
#[must_use]
pub fn fspl(frequency: f32, distance: f32) -> f32 {
    -27.55
        + 20.0 * frequency.log10()
        + if distance > 0.0 {
            20.0 * distance.log10()
        } else {
            0.0
        }
}

/// What every model needs about a transmitter/receiver pair.
#[derive(Debug, Clone, Copy)]
pub struct Link<'a> {
    pub tx_pos: Vec3,
    pub tx_dir: Vec3,
    pub rx_pos: Vec3,
    pub rx_dir: Vec3,
    pub tx_antenna: &'a crate::antenna::Antenna,
    pub rx_antenna: &'a crate::antenna::Antenna,
    /// MHz.
    pub frequency: f32,
    /// mW.
    pub power: f32,
    /// Terrain loss scaling (CBA setting).
    pub scale: f32,
    /// Ignore antenna patterns (CBA setting).
    pub omnidirectional: bool,
}

impl Link<'_> {
    /// (tx gain, rx gain) towards each other, or zero when omnidirectional.
    #[must_use]
    pub fn gains_towards(&self, tx_target: Vec3, rx_target: Vec3) -> (f32, f32) {
        if self.omnidirectional {
            return (0.0, 0.0);
        }
        (
            self.tx_antenna
                .gain(self.tx_dir, tx_target - self.tx_pos, self.frequency),
            self.rx_antenna
                .gain(self.rx_dir, rx_target - self.rx_pos, self.frequency),
        )
    }
}

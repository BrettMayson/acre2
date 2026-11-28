//! "Arcade": free-space loss plus a fixed 36 dB, and ±0.5 dB of noise. No terrain.

use std::cell::Cell;

use super::{Signal, dbm_to_v, fspl, mw_to_dbm};
use crate::math::Vec3;

thread_local! {
    static RNG: Cell<u64> = Cell::new(seed());
}

fn seed() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    // Mix in the thread so pool threads differ.
    let id = std::thread::current().id();
    let h = format!("{id:?}")
        .bytes()
        .fold(t, |h, b| h.rotate_left(5) ^ u64::from(b));
    h | 1
}

/// Uniform in [0, 1), like `rand() / (RAND_MAX + 1.0)`.
fn random() -> f32 {
    RNG.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 40) as f32 / (1u64 << 24) as f32
    })
}

#[must_use]
pub fn process(
    tx_pos: Vec3,
    rx_pos: Vec3,
    rx_antenna_name: &str,
    frequency: f32,
    power: f32,
) -> Signal {
    let mut loss_free_space = fspl(frequency, tx_pos.distance(rx_pos));
    if rx_antenna_name.contains("ACRE_2HALFINCH_UHF_TNC") {
        loss_free_space -= 17.0;
    }
    let tx_power = mw_to_dbm(power);
    let itu_loss = 36.0;
    let loss_tx = 3.0;
    let loss_rx = 3.0;
    let loss_model = itu_loss + (random() - 0.5);
    let budget = tx_power - loss_tx - loss_free_space - loss_model - loss_rx;
    Signal {
        dbm: budget,
        v: dbm_to_v(budget, 50.0),
        reflections: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_half_a_db() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1000.0, 0.0, 0.0);
        let base = mw_to_dbm(1000.0) - 6.0 - fspl(150.0, 1000.0) - 36.0;
        for _ in 0..1000 {
            let s = process(a, b, "ACRE_PRC152", 150.0, 1000.0);
            assert!((s.dbm - base).abs() <= 0.5 + 1e-4, "{} vs {base}", s.dbm);
        }
        let boosted = process(a, b, "ACRE_2HALFINCH_UHF_TNC", 150.0, 1000.0);
        assert!(boosted.dbm > base + 16.0);
    }
}

//! "LOS Simple": free-space loss plus knife-edge diffraction over the terrain peaks between
//! the two antennas (ITU single knife edge, summed).

use super::{Link, Signal, dbm_to_v, fspl};
use crate::{map::Map, math::Vec3};

const SAMPLE_SIZE: f32 = 7.5;

#[must_use]
pub fn process(map: &Map, link: &Link) -> Signal {
    let distance = link.tx_pos.distance(link.rx_pos);
    let tx_power = 10.0 * (link.power / 1000.0).log10() + 30.0;
    let (tx_gain, rx_gain) = link.gains_towards(link.rx_pos, link.tx_pos);
    let budget = tx_power + tx_gain
        - link.tx_antenna.internal_loss_dbm
        - fspl(link.frequency, distance)
        - diffraction_loss(map, link.tx_pos, link.rx_pos, link.frequency) * link.scale
        + rx_gain
        - link.rx_antenna.internal_loss_dbm;
    Signal {
        dbm: budget,
        v: dbm_to_v(budget, 50.0),
        reflections: Vec::new(),
    }
}

/// Loss over every local terrain maximum along the path.
#[must_use]
pub fn diffraction_loss(map: &Map, pos1: Vec3, pos2: Vec3, frequency: f32) -> f32 {
    if pos1.distance(pos2) <= 0.0 {
        return 0.0;
    }
    let profile = map.terrain_profile(pos1, pos2, SAMPLE_SIZE);
    let dir = (pos2 - pos1).normalize() * SAMPLE_SIZE;
    let total_2d = pos1.distance_2d(pos2);
    if profile.len() < 4 {
        return 0.0;
    }
    let mut loss = 0.0;
    for c in 0..profile.len() - 1 {
        let sample = profile[c];
        let last = profile[c.saturating_sub(1)];
        let next = profile[c + 1];
        if last < sample && next < sample {
            let ray = pos1 + dir * c as f32;
            let d1 = pos1.distance_2d(ray);
            let d2 = total_2d - d1;
            loss += itu(ray.z - sample, d1 / 1000.0, d2 / 1000.0, frequency / 1000.0);
        }
    }
    loss
}

fn itu(h: f32, d1_km: f32, d2_km: f32, f_ghz: f32) -> f32 {
    let d = d1_km + d2_km;
    let f1 = 17.3 * ((d1_km * d2_km) / (f_ghz * d)).sqrt();
    let a = -20.0 * h / f1 + 10.0;
    if a < 6.0 { 0.0 } else { a }
}

//! "LOS Multipath" (the default): the direct path plus up to 10 reflections off terrain peaks
//! near the transmitter, summed with their phases.

use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use super::{Link, Reflection, Signal, dbm_to_v, fspl, los::diffraction_loss, v_to_dbm};
use crate::{map::Map, math::Vec3};

const MAGIC: f32 = 10.0;

/// Peaks near a transmitter position.
type PeakCache = Option<((f32, f32), Arc<Vec<Vec3>>)>;

#[derive(Debug)]
pub struct Multipath {
    map: Arc<Map>,
    grid_size: i32,
    buckets: Vec<Vec<Vec3>>,
    /// Peaks near the last transmitter position. (The C++ appended to this list every time the
    /// transmitter moved and never cleared it, so results depended on call history.)
    cache: Mutex<PeakCache>,
}

impl Multipath {
    #[must_use]
    pub fn new(map: Arc<Map>) -> Self {
        let grid_size = (map.size() as f32 * map.cell_size() / 1000.0).ceil() as i32;
        let mut buckets = vec![Vec::new(); (grid_size * grid_size).max(0) as usize];
        for peak in &map.peaks {
            let x = (peak.x / 1000.0).floor() as i32;
            let y = (peak.y / 1000.0).floor() as i32;
            if (0..grid_size).contains(&x) && (0..grid_size).contains(&y) {
                buckets[(x * grid_size + y) as usize].push(*peak);
            }
        }
        Self {
            map,
            grid_size,
            buckets,
            cache: Mutex::new(None),
        }
    }

    fn peaks_near(&self, x: f32, y: f32, size: i32) -> Arc<Vec<Vec3>> {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((pos, peaks)) = &*cache
            && *pos == (x, y)
        {
            return peaks.clone();
        }
        let peaks = Arc::new(self.peaks_spiral(x, y, size, size));
        *cache = Some(((x, y), peaks.clone()));
        peaks
    }

    /// Peaks in the 1 km buckets around (x, y), nearest first (a square spiral).
    fn peaks_spiral(&self, pos_x: f32, pos_y: f32, size_x: i32, size_y: i32) -> Vec<Vec3> {
        let gx = (pos_x / 1000.0).floor() as i32;
        let gy = (pos_y / 1000.0).floor() as i32;
        let (mut x, mut y, mut dx, mut dy) = (0i32, 0i32, 0i32, -1i32);
        let t = size_x.max(size_y);
        let mut out = Vec::new();
        for _ in 0..t * t {
            if -size_x / 2 <= x && x <= size_x / 2 && -size_y / 2 <= y && y <= size_y / 2 {
                let (bx, by) = (gx + x, gy + y);
                // The C++ skipped the last row and column too.
                if bx >= 0 && bx < self.grid_size - 1 && by >= 0 && by < self.grid_size - 1 {
                    out.extend_from_slice(&self.buckets[(bx * self.grid_size + by) as usize]);
                }
            }
            if x == y || (x < 0 && x == -y) || (x > 0 && x == 1 - y) {
                let t = dx;
                dx = -dy;
                dy = t;
            }
            x += dx;
            y += dy;
        }
        out
    }

    #[must_use]
    pub fn process(&self, link: &Link) -> Signal {
        let map = &*self.map;
        let mut result = Signal::default();
        let tx_power = 10.0 * (link.power / 1000.0).log10() + 30.0;
        let distance = link.tx_pos.distance(link.rx_pos);
        let mut search_size = (search_distance(link.frequency, link.power) * 2.0 / 1000.0) as i32;
        if search_size % 2 == 0 {
            search_size += 1;
        }
        let peaks = self.peaks_near(link.tx_pos.x, link.tx_pos.y, search_size);

        let mut signals = Vec::new();
        let mut phases = Vec::new();
        let mut good = 0;
        let mut best_signal = 0.0f32;
        let mut best_index = 0usize;
        let tx_loss = link.tx_antenna.internal_loss_dbm;
        let rx_loss = link.rx_antenna.internal_loss_dbm;

        for &peak in peaks.iter() {
            let v_tx = link.tx_pos - peak;
            let v_rx = link.rx_pos - peak;
            // The C++'s truncated √2, kept for identical results.
            #[allow(clippy::approx_constant)]
            if v_tx.normalize().distance(v_rx.normalize()) > 1.414_21 {
                continue;
            }
            let v_mid = (v_tx + v_rx).normalize();
            let mut best_point = Vec3::default();
            let mut best_distance = 10000.0f32;
            for i in 0..=40 {
                let test = peak + v_mid * i as f32 * 7.5;
                let d = map.normal(test.x, test.y).distance2(v_mid);
                if d < best_distance {
                    best_distance = d;
                    best_point = test;
                }
            }
            best_point.z = map.elevation(best_point.x, best_point.y);
            let (tx_gain, rx_gain) = link.gains_towards(best_point, best_point);
            let path =
                (link.tx_pos.distance2(best_point) + link.rx_pos.distance2(best_point)).sqrt();
            let path_loss = fspl(link.frequency, path);
            let to_rx = (link.rx_pos - best_point).normalize();
            let normal = map.normal(best_point.x, best_point.y);
            let ratio = normal.dot(to_rx.normalize());
            let tx_power_reflected = 10.0 * ((link.power * ratio) / 1000.0).log10() + 30.0;
            let mut budget = tx_power_reflected + tx_gain
                - tx_loss
                - path_loss
                - (-27.55 + 20.0 * link.frequency.log10())
                + rx_gain
                - rx_loss;
            if budget > -200.0 {
                let mut loss =
                    diffraction_loss(map, link.tx_pos, best_point, link.frequency) * link.scale;
                if budget - loss > -200.0 {
                    loss +=
                        diffraction_loss(map, best_point, link.rx_pos, link.frequency) * link.scale;
                    budget -= loss;
                    if budget > -200.0 {
                        let v = dbm_to_v(budget, 50.0);
                        signals.push(v);
                        let p = phase(path, link.frequency) + PI;
                        phases.push(p);
                        result.reflections.push(Reflection {
                            point: best_point,
                            normal,
                            phase: p,
                            reflect_coefficient: ratio,
                            path_budget_dbm: budget,
                            path_budget_v: v,
                        });
                        if v > best_signal {
                            best_signal = v;
                            best_index = signals.len() - 1;
                        }
                        good += 1;
                    }
                }
            }
            if good >= 10 {
                break;
            }
        }

        let (tx_gain, rx_gain) = link.gains_towards(link.rx_pos, link.tx_pos);
        let budget = tx_power + tx_gain
            - tx_loss
            - fspl(link.frequency, distance)
            - MAGIC
            - diffraction_loss(map, link.tx_pos, link.rx_pos, link.frequency) * link.scale
            + rx_gain
            - rx_loss;
        let v = dbm_to_v(budget, 50.0);
        signals.push(v);
        phases.push(phase(distance, link.frequency));
        if v > best_signal {
            best_signal = v;
            best_index = signals.len() - 1;
        }

        let total = if link.scale >= 1.0 {
            let best_phase = phases[best_index];
            let mut total = best_signal;
            for (c, &level) in signals.iter().enumerate() {
                if c != best_index {
                    let mut p = phases[c] - best_phase;
                    if p > PI {
                        p -= PI * 2.0;
                    } else if p < -PI {
                        p += PI * 2.0;
                    }
                    total = (total * total + level * level + 2.0 * total * level * p.cos()).sqrt();
                }
            }
            total
        } else {
            best_signal
        };
        if total > 0.0 {
            result.v = total;
            result.dbm = v_to_dbm(total, 50.0);
        } else {
            result.v = 0.0;
            result.dbm = -1000.0;
        }
        result
    }
}

/// How far (in 250 m steps) a transmitter could plausibly reach.
fn search_distance(frequency: f32, power: f32) -> f32 {
    let tx_power = 10.0 * (power / 1000.0).log10() + 30.0;
    let mut test = 0.0f32;
    let mut c = 0;
    let mut distance = 0.0;
    while test < 80.0 + tx_power {
        distance = c as f32 * 250.0;
        test = (-27.55 + 20.0 * frequency.log10() + 20.0 * distance.log10()) + MAGIC * 3.0;
        c += 1;
    }
    distance
}

fn phase(path_distance: f32, f_mhz: f32) -> f32 {
    let p = PI * 2.0 * path_distance / (300.0 / f_mhz);
    p % (PI * 2.0) - PI
}

//! Longley-Rice Irregular Terrain Model (ITM), point-to-point mode.
//!
//! A port of the public-domain ITM (NTIA) as the C++ carried it (`longleyRice_itm.cpp`, the
//! SPLAT!/Holger Schemel cleanup). The C++ kept intermediate values in `static thread_local`
//! variables that each call re-initialises; here they live in [`Ctx`] for one calculation.
//! Comments with `[Alg x.y]` refer to the ITM algorithm document.

// Kept as close to the C++ as possible: its constants (1.4142…), its min/max order (not
// `clamp`, which differs on NaN), its `(a + b) / 2` and its repeated `kwx = 4` branches.
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::manual_midpoint,
    clippy::manual_clamp,
    clippy::items_after_statements,
    clippy::if_same_then_else
)]

use std::f64::consts::{FRAC_PI_2, PI};

use super::{Link, Signal, dbm_to_v, mw_to_dbm};
use crate::{antenna::Polarization, map::Map, math::Vec3};

const THIRD: f64 = 1.0 / 3.0;
const F_0: f64 = 47.7;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }
    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }
    fn scale(self, s: f64) -> Self {
        Self::new(self.re * s, self.im * s)
    }
    fn div(self, o: Self) -> Self {
        let d = o.re * o.re + o.im * o.im;
        Self::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn norm(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    fn sqrt(self) -> Self {
        // Principal square root, as csqrt.
        let m = self.abs();
        let re = ((m + self.re) / 2.0).sqrt();
        let im = ((m - self.re) / 2.0).sqrt().copysign(self.im);
        Self::new(re, im)
    }
}

#[derive(Debug, Default, Clone)]
struct Prop {
    d: f64,
    h_g: [f64; 2],
    k: f64,
    delta_h: f64,
    n_s: f64,
    gamma_e: f64,
    z_g: Complex,
    h_e: [f64; 2],
    d_lj: [f64; 2],
    theta_ej: [f64; 2],
    mdp: i32,
    kwx: i32,
    a_ref: f64,
    dx: f64,
    ael: f64,
    ak1: f64,
    ak2: f64,
    aed: f64,
    emd: f64,
    aes: f64,
    ems: f64,
    d_lsj: [f64; 2],
    d_ls: f64,
    d_l: f64,
    theta_e: f64,
}

impl Default for Complex {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

#[derive(Debug, Default, Clone)]
struct PropV {
    lvar: i32,
    mdvar: i32,
    klim: i32,
    sgc: f64,
}

/// The C++'s `static thread_local` state.
#[derive(Debug, Default)]
struct Ctx {
    // adiff
    wd1: f64,
    xd1: f64,
    a_fo: f64,
    qk: f64,
    aht: f64,
    xht: f64,
    // ascat
    ad: f64,
    rr: f64,
    etq: f64,
    h0s: f64,
    // alos
    wls: f64,
    // lrprop
    wlos: bool,
    wscat: bool,
    dmin: f64,
    xae: f64,
    // avar
    avar: AvarState,
}

#[derive(Debug, Default)]
struct AvarState {
    kdv: i32,
    dexa: f64,
    de: f64,
    vmd: f64,
    vs0: f64,
    sgl: f64,
    sgtm: f64,
    sgtp: f64,
    sgtd: f64,
    tgtd: f64,
    gm: f64,
    gp: f64,
    cv1: f64,
    cv2: f64,
    yv1: f64,
    yv2: f64,
    yv3: f64,
    csm1: f64,
    csm2: f64,
    ysm1: f64,
    ysm2: f64,
    ysm3: f64,
    csp1: f64,
    csp2: f64,
    ysp1: f64,
    ysp2: f64,
    ysp3: f64,
    csd1: f64,
    zd: f64,
    cfm1: f64,
    cfm2: f64,
    cfm3: f64,
    cfp1: f64,
    cfp2: f64,
    cfp3: f64,
    no_location_variability: bool,
    no_situation_variability: bool,
}

fn dim(x: f64, y: f64) -> f64 {
    if x > y { x - y } else { 0.0 }
}

/// Single knife-edge attenuation, as a function of v².
fn fn_knife(v_square: f64) -> f64 {
    if v_square <= 5.76 {
        6.02 + 9.11 * v_square.sqrt() - 1.27 * v_square
    } else {
        12.953 + 4.343 * v_square.ln()
    }
}

/// Height gain over a smooth spherical earth [Alg 6.4ff].
fn f_height_gain(x: f64, k: f64) -> f64 {
    if x <= 200.0 {
        let w = -k.ln();
        if k < 1e-5 || x * w * w * w > 5495.0 {
            let mut f = -117.0;
            if x > 1.0 {
                f += 17.372 * x.ln();
            }
            f
        } else {
            2.5e-5 * x * x / k - 8.686 * w - 15.0
        }
    } else {
        let mut f = 0.05751 * x - 4.343 * x.ln();
        if x < 2000.0 {
            let w = 0.0134 * x * (-0.005 * x).exp();
            f = (1.0 - w) * f + w * (17.372 * x.ln() - 117.0);
        }
        f
    }
}

fn h_0(r: f64, et: f64) -> f64 {
    const A: [f64; 5] = [25.0, 80.0, 177.0, 395.0, 705.0];
    const B: [f64; 5] = [24.0, 45.0, 68.0, 80.0, 105.0];
    let mut it = et as i32;
    let q;
    if it <= 0 {
        it = 1;
        q = 0.0;
    } else if it >= 4 {
        it = 4;
        q = 0.0;
    } else {
        q = et - f64::from(it);
    }
    let it = it as usize;
    let mut x = 1.0 / r;
    x *= x;
    let mut h = 4.343 * ((1.0 + A[it - 1] * x + B[it - 1]) * x).ln();
    if q != 0.0 {
        h = (1.0 - q) * h + q * 4.343 * ((A[it] * x + B[it]) * x + 1.0).ln();
    }
    h
}

fn f_0(td: f64) -> f64 {
    if td <= 10e3 {
        133.4 + 104.6 * td + 71.8 * td.ln()
    } else if td <= 70e3 {
        0.332e-3 + 0.212e-3 * td + 0.157e-3 * td.ln()
    } else {
        -4.343 + -1.086 * td + 2.171 * td.ln()
    }
}

/// Diffraction attenuation at distance `s`; `s == 0` sets up.
fn adiff(s: f64, prop: &Prop, c: &mut Ctx) -> f64 {
    const A: f64 = 151.03;
    const D: f64 = 50e3;
    const H: f64 = 16.0;
    const ALPHA: f64 = 4.77e-4;
    if s == 0.0 {
        let mut q = prop.h_g[0] * prop.h_g[1];
        c.qk = prop.h_e[0] * prop.h_e[1] - q;
        if prop.mdp < 0 {
            q += 10.0;
        }
        c.wd1 = (1.0 + c.qk / q).sqrt();
        c.xd1 = prop.d_l + prop.theta_e / prop.gamma_e;
        q = (1.0 - 0.8 * (-prop.d_ls / D).exp()) * prop.delta_h;
        q *= 0.78 * (-(q / H).powf(0.25)).exp();
        c.a_fo = 15f64.min(2.171 * (1.0 + ALPHA * prop.h_g[0] * prop.h_g[1] * prop.k * q).ln());
        c.qk = 1.0 / prop.z_g.abs();
        c.aht = 20.0;
        c.xht = 0.0;
        for j in 0..2 {
            let gamma_j_recip = 0.5 * (prop.d_lj[j] * prop.d_lj[j]) / prop.h_e[j];
            let alpha = (gamma_j_recip * prop.k).powf(THIRD);
            let k = c.qk / alpha;
            let q = A * (1.607 - k) * alpha * prop.d_lj[j] / gamma_j_recip;
            c.xht += q;
            c.aht += f_height_gain(q, k);
        }
        return 0.0;
    }
    let theta = prop.theta_e + s * prop.gamma_e;
    let ds = s - prop.d_l;
    let mut q = 0.079_577_5 * prop.k * ds * theta * theta;
    let a_k = fn_knife(q * prop.d_lj[0] / (ds + prop.d_lj[0]))
        + fn_knife(q * prop.d_lj[1] / (ds + prop.d_lj[1]));
    let gamma_o_recip = ds / theta;
    let alpha = (gamma_o_recip * prop.k).powf(THIRD);
    let k = c.qk / alpha;
    q = A * (1.607 - k) * alpha * theta + c.xht;
    let a_r = 0.05751 * q - 4.343 * q.ln() - c.aht;
    q = (c.wd1 + c.xd1 / s) * ((1.0 - 0.8 * (-s / 50e3).exp()) * prop.delta_h * prop.k).min(6283.2);
    let w = 25.1 / (25.1 + q.sqrt());
    (1.0 - w) * a_k + w * a_r + c.a_fo
}

/// Scatter attenuation at distance `s`; `s == 0` sets up.
fn a_scat(s: f64, prop: &Prop, c: &mut Ctx) -> f64 {
    if s == 0.0 {
        c.ad = prop.d_lj[0] - prop.d_lj[1];
        c.rr = prop.h_e[1] / prop.h_e[0];
        if c.ad < 0.0 {
            c.ad = -c.ad;
            c.rr = 1.0 / c.rr;
        }
        c.etq = (5.67e-6 * prop.n_s - 2.32e-3) * prop.n_s + 0.031;
        c.h0s = -15.0;
        return 0.0;
    }
    let h0 = if c.h0s > 15.0 {
        c.h0s
    } else {
        let theta_tick = prop.theta_ej[0] + prop.theta_ej[1] + prop.gamma_e * s;
        let mut r2 = 2.0 * prop.k * theta_tick;
        let r1 = r2 * prop.h_e[0];
        r2 *= prop.h_e[1];
        if r1 < 0.2 && r2 < 0.2 {
            return 1001.0;
        }
        let mut ss = (s - c.ad) / (s + c.ad);
        let mut q = c.rr / ss;
        ss = ss.max(0.1);
        q = q.max(0.1).min(10.0);
        let z0 = (s - c.ad) * (s + c.ad) * theta_tick * 0.25 / s;
        let mut temp = (z0 / 8.0e3).min(1.7);
        temp = temp * temp * temp * temp * temp * temp;
        let et = (c.etq * (-temp).exp() + 1.0) * z0 / 1.7556e3;
        let ett = et.max(1.0);
        let mut h0 = (h_0(r1, ett) + h_0(r2, ett)) * 0.5;
        h0 += h0.min((1.38 - ett.ln()) * ss.ln() * q.ln() * 0.49);
        h0 = dim(h0, 0.0);
        if et < 1.0 {
            h0 = et * h0
                + (1.0 - et)
                    * 4.343
                    * (((1.0 + 1.4142 / r1) * (1.0 + 1.4142 / r2)).powf(2.0) * (r1 + r2)
                        / (r1 + r2 + 2.8284))
                        .ln();
        }
        if h0 > 15.0 && c.h0s >= 0.0 {
            h0 = c.h0s;
        }
        h0
    };
    c.h0s = h0;
    let theta = prop.theta_e + s * prop.gamma_e;
    const D_0: f64 = 40e3;
    const H: f64 = 47.7;
    4.343 * (prop.k * H * theta * theta * theta * theta).ln() + f_0(theta * s)
        - 0.1 * (prop.n_s - 301.0) * (-theta * s / D_0).exp()
        + h0
}

/// Line-of-sight attenuation at distance `d`; `d == 0` sets up.
fn a_los(d: f64, prop: &Prop, c: &mut Ctx) -> f64 {
    if d == 0.0 {
        const D1: f64 = 47.7;
        const D2: f64 = 10e3;
        let d1r = 1.0 / D1;
        c.wls = d1r / (d1r + prop.k * prop.delta_h / D2.max(prop.d_ls));
        return 0.0;
    }
    const D: f64 = 50e3;
    const H: f64 = 16.0;
    let mut q = (1.0 - 0.8 * (-d / D).exp()) * prop.delta_h;
    let s = 0.78 * q * (-(q / H).powf(0.25)).exp();
    q = prop.h_e[0] + prop.h_e[1];
    let sps = q / (d * d + q * q).sqrt();
    let sps_c = Complex::new(sps, 0.0);
    let mut r = sps_c
        .sub(prop.z_g)
        .div(sps_c.add(prop.z_g))
        .scale((-(10f64.min(prop.k * s * sps))).exp());
    q = r.norm();
    if q < 0.25 || q < sps {
        r = r.scale((sps / q).sqrt());
    }
    let alosv = prop.emd * d + prop.aed;
    q = prop.k * prop.h_e[0] * prop.h_e[1] * 2.0 / d;
    if q > FRAC_PI_2 {
        q = PI - (FRAC_PI_2 * FRAC_PI_2) / q;
    }
    (-4.343 * Complex::new(q.cos(), -q.sin()).add(r).norm().ln() - alosv) * c.wls + alosv
}

fn lrprop(d: f64, prop: &mut Prop, c: &mut Ctx) {
    if prop.mdp != 0 {
        for j in 0..2 {
            prop.d_lsj[j] = (2.0 * prop.h_e[j] / prop.gamma_e).sqrt();
        }
        prop.d_ls = prop.d_lsj[0] + prop.d_lsj[1];
        prop.d_l = prop.d_lj[0] + prop.d_lj[1];
        prop.theta_e = (prop.theta_ej[0] + prop.theta_ej[1]).max(-prop.d_l * prop.gamma_e);
        c.wlos = false;
        c.wscat = false;
        if prop.k < 0.838 || prop.k > 210.0 {
            prop.kwx = prop.kwx.max(1);
        }
        if prop.n_s < 250.0 || prop.n_s > 400.0 {
            prop.kwx = 4;
        } else if prop.gamma_e < 75e-9 || prop.gamma_e > 250e-9 {
            prop.kwx = 4;
        } else if prop.z_g.re <= prop.z_g.im.abs() {
            prop.kwx = 4;
        } else if prop.k < 0.419 || prop.k > 420.0 {
            prop.kwx = 4;
        } else {
            for j in 0..2 {
                if prop.h_g[j] < 1.0 || prop.h_g[j] > 1000.0 {
                    prop.kwx = prop.kwx.max(1);
                }
                if prop.theta_ej[j].abs() > 200e-3 {
                    prop.kwx = prop.kwx.max(3);
                }
                if prop.d_lj[j] < 0.1 * prop.d_lsj[j] || prop.d_lj[j] > 3.0 * prop.d_lsj[j] {
                    prop.kwx = prop.kwx.max(3);
                }
                if prop.h_g[j] < 0.5 || prop.h_g[j] > 3000.0 {
                    prop.kwx = 4;
                }
            }
        }
        c.dmin = (prop.h_e[0] - prop.h_e[1]).abs() / 200e-3;
        adiff(0.0, prop, c);
        c.xae = (prop.k * prop.gamma_e * prop.gamma_e).powf(-THIRD);
        let d3 = prop.d_ls.max(1.3787 * c.xae + prop.d_l);
        let d4 = d3 + 2.7574 * c.xae;
        let a3 = adiff(d3, prop, c);
        let a4 = adiff(d4, prop, c);
        prop.emd = (a4 - a3) / (d4 - d3);
        prop.aed = a3 - prop.emd * d3;
    }
    if prop.mdp >= 0 {
        prop.mdp = 0;
        prop.d = d;
    }
    if prop.d > 0.0 {
        if prop.d > 1000e3 {
            prop.kwx = prop.kwx.max(1);
        }
        if prop.d < c.dmin {
            prop.kwx = prop.kwx.max(3);
        }
        if prop.d < 1e3 || prop.d > 2000e3 {
            prop.kwx = 4;
        }
    }
    if prop.d < prop.d_ls {
        if !c.wlos {
            a_los(0.0, prop, c);
            let d2 = prop.d_ls;
            let a2 = prop.aed + d2 * prop.emd;
            let mut d0 = 1.908 * prop.k * prop.h_e[0] * prop.h_e[1];
            let d1 = if prop.aed >= 0.0 {
                d0 = d0.min(0.5 * prop.d_l);
                d0 + 0.25 * (prop.d_l - d0)
            } else {
                (-prop.aed / prop.emd).max(0.25 * prop.d_l)
            };
            let a1 = a_los(d1, prop, c);
            let mut wq = false;
            if d0 < d1 {
                let a0 = a_los(d0, prop, c);
                let q = (d2 / d0).ln();
                prop.ak2 = 0f64.max(
                    ((d2 - d0) * (a1 - a0) - (d1 - d0) * (a2 - a0))
                        / ((d2 - d0) * (d1 / d0).ln() - (d1 - d0) * q),
                );
                wq = prop.aed >= 0.0 || prop.ak2 > 0.0;
                if wq {
                    prop.ak1 = (a2 - a0 - prop.ak2 * q) / (d2 - d0);
                    if prop.ak1 < 0.0 {
                        prop.ak1 = 0.0;
                        prop.ak2 = dim(a2, a0) / q;
                        if prop.ak2 == 0.0 {
                            prop.ak1 = prop.emd;
                        }
                    }
                }
            }
            if !wq {
                prop.ak1 = dim(a2, a1) / (d2 - d1);
                prop.ak2 = 0.0;
                if prop.ak1 == 0.0 {
                    prop.ak1 = prop.emd;
                }
            }
            prop.ael = a2 - prop.ak1 * d2 - prop.ak2 * d2.ln();
            c.wlos = true;
        }
        if prop.d > 0.0 {
            prop.a_ref = prop.ael + prop.ak1 * prop.d + prop.ak2 * prop.d.ln();
        }
    }
    if prop.d <= 0.0 || prop.d >= prop.d_ls {
        if !c.wscat {
            const DS: f64 = 200e3;
            const HS: f64 = 47.7;
            a_scat(0.0, prop, c);
            let d5 = prop.d_l + DS;
            let d6 = d5 + DS;
            let a6 = a_scat(d6, prop, c);
            let a5 = a_scat(d5, prop, c);
            if a5 < 1000.0 {
                prop.ems = (a6 - a5) / DS;
                prop.dx = prop.d_ls.max(
                    (prop.d_l + 0.3 * c.xae * (HS * prop.k).ln())
                        .max((a5 - prop.aed - prop.ems * d5) / (prop.emd - prop.ems)),
                );
                prop.aes = (prop.emd - prop.ems) * prop.dx + prop.aed;
            } else {
                prop.ems = prop.emd;
                prop.aes = prop.aed;
                prop.dx = 10.0e6;
            }
            c.wscat = true;
        }
        prop.a_ref = if prop.d > prop.dx {
            prop.aes + prop.ems * prop.d
        } else {
            prop.aed + prop.emd * prop.d
        };
    }
    prop.a_ref = prop.a_ref.max(0.0);
}

/// Inverse of the complementary normal distribution (Hastings).
fn qerfi(q: f64) -> f64 {
    const C0: f64 = 2.515_516_698;
    const C1: f64 = 0.802_853;
    const C2: f64 = 0.010_328;
    const D1: f64 = 1.432_788;
    const D2: f64 = 0.189_269;
    const D3: f64 = 0.001_308;
    let x = 0.5 - q;
    let mut t = (0.5 - x.abs()).max(0.000_001);
    t = (-2.0 * t.ln()).sqrt();
    let mut v = t - ((C2 * t + C1) * t + C0) / (((D3 * t + D2) * t + D1) * t + 1.0);
    if x < 0.0 {
        v = -v;
    }
    v
}

fn qlrps(fmhz: f64, zsys: f64, en0: f64, ipol: i32, eps: f64, sgm: f64, prop: &mut Prop) {
    const Z_1: f64 = 9.46e3;
    const GMA: f64 = 157e-9;
    const N_1: f64 = 179.3;
    const Z_0: f64 = 376.62;
    prop.k = fmhz / F_0;
    prop.n_s = en0;
    if zsys != 0.0 {
        prop.n_s *= (-zsys / Z_1).exp();
    }
    prop.gamma_e = GMA * (1.0 - 0.04665 * (prop.n_s / N_1).exp());
    let zq = Complex::new(eps, Z_0 * sgm / prop.k);
    let mut z = zq.sub(Complex::new(1.0, 0.0)).sqrt();
    if ipol != 0 {
        z = z.div(zq);
    }
    prop.z_g = z;
}

fn curve(c1: f64, c2: f64, x1: f64, x2: f64, x3: f64, de: f64) -> f64 {
    let mut t1 = (de - x2) / x3;
    let mut t2 = de / x1;
    t1 *= t1;
    t2 *= t2;
    (c1 + c2 / (1.0 + t1)) * t2 / (1.0 + t2)
}

#[allow(clippy::too_many_lines)]
fn avar(
    zzt: f64,
    zzl: f64,
    zzc: f64,
    prop: &mut Prop,
    propv: &mut PropV,
    s: &mut AvarState,
) -> f64 {
    const BV1: [f64; 7] = [-9.67, -0.62, 1.26, -9.21, -0.62, -0.39, 3.15];
    const BV2: [f64; 7] = [12.7, 9.19, 15.5, 9.05, 9.19, 2.86, 857.9];
    const XV1: [f64; 7] = [
        144.9e3, 228.9e3, 262.6e3, 84.1e3, 228.9e3, 141.7e3, 2222.0e3,
    ];
    const XV2: [f64; 7] = [
        190.3e3, 205.2e3, 185.2e3, 101.1e3, 205.2e3, 315.9e3, 164.8e3,
    ];
    const XV3: [f64; 7] = [133.8e3, 143.6e3, 99.8e3, 98.6e3, 143.6e3, 167.4e3, 116.3e3];
    const BSM1: [f64; 7] = [2.13, 2.66, 6.11, 1.98, 2.68, 6.86, 8.51];
    const BSM2: [f64; 7] = [159.5, 7.67, 6.65, 13.11, 7.16, 10.38, 169.8];
    const XSM1: [f64; 7] = [762.2e3, 100.4e3, 138.2e3, 139.1e3, 93.7e3, 187.8e3, 609.8e3];
    const XSM2: [f64; 7] = [
        123.6e3, 172.5e3, 242.2e3, 132.7e3, 186.8e3, 169.6e3, 119.9e3,
    ];
    const XSM3: [f64; 7] = [94.5e3, 136.4e3, 178.6e3, 193.5e3, 133.5e3, 108.9e3, 106.6e3];
    const BSP1: [f64; 7] = [2.11, 6.87, 10.08, 3.68, 4.75, 8.58, 8.43];
    const BSP2: [f64; 7] = [102.3, 15.53, 9.60, 159.3, 8.12, 13.97, 8.19];
    const XSP1: [f64; 7] = [636.9e3, 138.7e3, 165.3e3, 464.4e3, 93.2e3, 216.0e3, 136.2e3];
    const XSP2: [f64; 7] = [134.8e3, 143.7e3, 225.7e3, 93.1e3, 135.9e3, 152.0e3, 188.5e3];
    const XSP3: [f64; 7] = [95.6e3, 98.6e3, 129.7e3, 94.2e3, 113.4e3, 122.7e3, 122.9e3];
    const BSD1: [f64; 7] = [1.224, 0.801, 1.380, 1.000, 1.224, 1.518, 1.518];
    const BZD1: [f64; 7] = [1.282, 2.161, 1.282, 20.0, 1.282, 1.282, 1.282];
    const BFM1: [f64; 7] = [1.0, 1.0, 1.0, 1.0, 0.92, 1.0, 1.0];
    const BFM2: [f64; 7] = [0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0];
    const BFM3: [f64; 7] = [0.0, 0.0, 0.0, 0.0, 1.77, 0.0, 0.0];
    const BFP1: [f64; 7] = [1.0, 0.93, 1.0, 0.93, 0.93, 1.0, 1.0];
    const BFP2: [f64; 7] = [0.0, 0.31, 0.0, 0.19, 0.31, 0.0, 0.0];
    const BFP3: [f64; 7] = [0.0, 2.00, 0.0, 1.79, 2.00, 0.0, 0.0];
    const RT: f64 = 7.8;
    const RL: f64 = 24.0;

    if propv.lvar > 0 {
        // The C++ switch fell through from `default` down to `case 1`.
        if propv.lvar >= 5 {
            if propv.klim <= 0 || propv.klim > 7 {
                propv.klim = 5;
                prop.kwx = prop.kwx.max(2);
            }
            let i = (propv.klim - 1) as usize;
            s.cv1 = BV1[i];
            s.cv2 = BV2[i];
            s.yv1 = XV1[i];
            s.yv2 = XV2[i];
            s.yv3 = XV3[i];
            s.csm1 = BSM1[i];
            s.csm2 = BSM2[i];
            s.ysm1 = XSM1[i];
            s.ysm2 = XSM2[i];
            s.ysm3 = XSM3[i];
            s.csp1 = BSP1[i];
            s.csp2 = BSP2[i];
            s.ysp1 = XSP1[i];
            s.ysp2 = XSP2[i];
            s.ysp3 = XSP3[i];
            s.csd1 = BSD1[i];
            s.zd = BZD1[i];
            s.cfm1 = BFM1[i];
            s.cfm2 = BFM2[i];
            s.cfm3 = BFM3[i];
            s.cfp1 = BFP1[i];
            s.cfp2 = BFP2[i];
            s.cfp3 = BFP3[i];
        }
        if propv.lvar >= 4 {
            s.kdv = propv.mdvar;
            s.no_situation_variability = s.kdv >= 20;
            if s.no_situation_variability {
                s.kdv -= 20;
            }
            s.no_location_variability = s.kdv >= 10;
            if s.no_location_variability {
                s.kdv -= 10;
            }
            if s.kdv < 0 || s.kdv > 3 {
                s.kdv = 0;
                prop.kwx = prop.kwx.max(2);
            }
        }
        if propv.lvar >= 3 {
            let q = (0.133 * prop.k).ln();
            s.gm = s.cfm1 + s.cfm2 / ((s.cfm3 * q * s.cfm3 * q) + 1.0);
            s.gp = s.cfp1 + s.cfp2 / ((s.cfp3 * q * s.cfp3 * q) + 1.0);
        }
        if propv.lvar >= 2 {
            const A_1: f64 = 9000e3;
            s.dexa = (2.0 * A_1 * prop.h_e[0]).sqrt()
                + (2.0 * A_1 * prop.h_e[1]).sqrt()
                + (575.7e12 / prop.k).powf(THIRD);
        }
        {
            const D_0: f64 = 130e3;
            s.de = if prop.d < s.dexa {
                D_0 * prop.d / s.dexa
            } else {
                D_0 + prop.d - s.dexa
            };
        }
        s.vmd = curve(s.cv1, s.cv2, s.yv1, s.yv2, s.yv3, s.de);
        s.sgtm = curve(s.csm1, s.csm2, s.ysm1, s.ysm2, s.ysm3, s.de) * s.gm;
        s.sgtp = curve(s.csp1, s.csp2, s.ysp1, s.ysp2, s.ysp3, s.de) * s.gp;
        s.sgtd = s.sgtp * s.csd1;
        s.tgtd = (s.sgtp - s.sgtd) * s.zd;
        s.sgl = if s.no_location_variability {
            0.0
        } else {
            let q = (1.0 - 0.8 * (-prop.d / 50e3).exp()) * prop.delta_h * prop.k;
            10.0 * q / (q + 13.0)
        };
        s.vs0 = if s.no_situation_variability {
            0.0
        } else {
            let v = 5.0 + 3.0 * (-s.de / 100e3).exp();
            v * v
        };
        propv.lvar = 0;
    }

    let mut zt = zzt;
    let mut zl = zzl;
    let zc = zzc;
    match s.kdv {
        0 => {
            zt = zc;
            zl = zc;
        }
        1 => zl = zc,
        2 => zl = zt,
        _ => {}
    }
    if zt.abs() > 3.1 || zl.abs() > 3.1 || zc.abs() > 3.1 {
        prop.kwx = prop.kwx.max(1);
    }
    let sgt = if zt < 0.0 {
        s.sgtm
    } else if zt <= s.zd {
        s.sgtp
    } else {
        s.sgtd + s.tgtd / zt
    };
    let vs =
        s.vs0 + (sgt * zt * sgt * zt) / (RT + zc * zc) + (s.sgl * zl * s.sgl * zl) / (RL + zc * zc);
    let yr;
    match s.kdv {
        0 => {
            yr = 0.0;
            propv.sgc = (sgt * sgt + s.sgl * s.sgl + vs).sqrt();
        }
        1 => {
            yr = sgt * zt;
            propv.sgc = (s.sgl * s.sgl + vs).sqrt();
        }
        2 => {
            yr = (sgt * sgt + s.sgl * s.sgl).sqrt() * zt;
            propv.sgc = vs.sqrt();
        }
        _ => {
            yr = sgt * zt + s.sgl * zl;
            propv.sgc = vs.sqrt();
        }
    }
    let mut v = prop.a_ref - s.vmd - yr - propv.sgc * zc;
    if v < 0.0 {
        v = v * (29.0 - v) / (29.0 - 10.0 * v);
    }
    v
}

/// `pfl[i]`, holding at the last value past the end (the C++ padded the profile instead).
fn at(pfl: &[f64], i: i64) -> f64 {
    let i = usize::try_from(i).unwrap_or(0).min(pfl.len() - 1);
    pfl[i]
}

/// Horizon distances and take-off angles from the profile.
fn hzns(pfl: &[f64], prop: &mut Prop) {
    let np = pfl[0] as i64;
    let xi = pfl[1];
    let za = at(pfl, 2) + prop.h_g[0];
    let zb = at(pfl, np + 2) + prop.h_g[1];
    let qc = 0.5 * prop.gamma_e;
    let mut q = qc * prop.d;
    prop.theta_ej[1] = (zb - za) / prop.d;
    prop.theta_ej[0] = prop.theta_ej[1] - q;
    prop.theta_ej[1] = -prop.theta_ej[1] - q;
    prop.d_lj = [prop.d, prop.d];
    if np >= 2 {
        let mut sa = 0.0;
        let mut sb = prop.d;
        let mut wq = true;
        for i in 1..np {
            sa += xi;
            sb -= xi;
            q = at(pfl, i + 2) - (qc * sa + prop.theta_ej[0]) * sa - za;
            if q > 0.0 {
                prop.theta_ej[0] += q / sa;
                prop.d_lj[0] = sa;
                wq = false;
            }
            if !wq {
                q = at(pfl, i + 2) - (qc * sb + prop.theta_ej[1]) * sb - zb;
                if q > 0.0 {
                    prop.theta_ej[1] += q / sb;
                    prop.d_lj[1] = sb;
                }
            }
        }
    }
}

/// Least-squares line through the profile between `x1` and `x2`: (z at start, z at end).
fn zlsq1(z: &[f64], x1: f64, x2: f64) -> (f64, f64) {
    let xn = z[0];
    let mut xa = f64::from(dim(x1 / z[1], 0.0) as i32);
    let mut xb = xn - f64::from(dim(xn, x2 / z[1]) as i32);
    if xb <= xa {
        xa = dim(xa, 1.0);
        xb = xn - dim(xn, xb + 1.0);
    }
    let mut ja = xa as i64;
    let jb = xb as i64;
    let n = jb - ja;
    xa = xb - xa;
    let mut x = -0.5 * xa;
    xb += x;
    let mut a = 0.5 * (at(z, ja + 2) + at(z, jb + 2));
    let mut b = 0.5 * (at(z, ja + 2) - at(z, jb + 2)) * x;
    for _ in 2..=n {
        ja += 1;
        x += 1.0;
        a += at(z, ja + 2);
        b += at(z, ja + 2) * x;
    }
    a /= xa;
    b = b * 12.0 / ((xa * xa + 2.0) * xa);
    (a - b * xb, a + b * (xn - xb))
}

/// Partial sort: the `ir`-th largest of `a[0..=nn]` (`a` is reordered).
fn qtile(nn: i64, a: &mut [f64], ir: i64) -> f64 {
    let mut q = 0.0;
    let (mut j1, mut i0) = (0i64, 0i64);
    let mut m = 0i64;
    let mut n = nn;
    let k = ir.max(0).min(n);
    let mut goto10 = true;
    let get = |a: &[f64], i: i64| a[i as usize];
    loop {
        if goto10 {
            q = get(a, k);
            i0 = m;
            j1 = n;
        }
        let mut i = i0;
        while i <= n && get(a, i) >= q {
            i += 1;
        }
        if i > n {
            i = n;
        }
        let mut j = j1;
        while j >= m && get(a, j) <= q {
            j -= 1;
        }
        if j < m {
            j = m;
        }
        if i < j {
            a.swap(i as usize, j as usize);
            i0 = i + 1;
            j1 = j - 1;
            goto10 = false;
        } else if i < k {
            a[k as usize] = get(a, i);
            a[i as usize] = q;
            m = i + 1;
            goto10 = true;
        } else if j > k {
            a[k as usize] = get(a, j);
            a[j as usize] = q;
            n = j - 1;
            goto10 = true;
        } else {
            return q;
        }
    }
}

/// Terrain irregularity Δh: the interdecile range of elevations between `x1` and `x2`.
fn dlthx(pfl: &[f64], x1: f64, x2: f64) -> f64 {
    let np = pfl[0] as i64;
    let mut xa = x1 / pfl[1];
    let mut xb = x2 / pfl[1];
    if xb - xa < 2.0 {
        return 0.0;
    }
    let ka = ((0.1 * (xb - xa + 8.0)) as i64).max(4).min(25);
    let n = 10 * ka - 5;
    let kb = n - ka + 1;
    let sn = (n - 1) as f64;
    let mut s = vec![0.0; (n + 2) as usize];
    s[0] = sn;
    s[1] = 1.0;
    xb = (xb - xa) / sn;
    let mut k = (xa + 1.0) as i64;
    xa -= k as f64;
    for j in 0..n {
        while xa > 0.0 && k < np {
            xa -= 1.0;
            k += 1;
        }
        s[(j + 2) as usize] = at(pfl, k + 2) + (at(pfl, k + 2) - at(pfl, k + 1)) * xa;
        xa += xb;
    }
    let (z0, zn) = zlsq1(&s, 0.0, sn);
    xa = z0;
    xb = (zn - xa) / sn;
    for j in 0..n {
        s[(j + 2) as usize] -= xa;
        xa += xb;
    }
    let hi = qtile(n - 1, &mut s[2..], ka - 1);
    let lo = qtile(n - 1, &mut s[2..], kb - 1);
    (hi - lo) / (1.0 - 0.8 * (-(x2 - x1) / 50.0e3).exp())
}

fn qlrpfl(pfl: &[f64], klimx: i32, mdvarx: i32, prop: &mut Prop, propv: &mut PropV, c: &mut Ctx) {
    prop.d = pfl[0] * pfl[1];
    let np = pfl[0] as i64;
    hzns(pfl, prop);
    let mut xl = [0.0; 2];
    for j in 0..2 {
        xl[j] = (15.0 * prop.h_g[j]).min(0.1 * prop.d_lj[j]);
    }
    xl[1] = prop.d - xl[1];
    prop.delta_h = dlthx(pfl, xl[0], xl[1]);
    if prop.d_lj[0] + prop.d_lj[1] > 1.5 * prop.d {
        let (za, zb) = zlsq1(pfl, xl[0], xl[1]);
        prop.h_e[0] = prop.h_g[0] + dim(at(pfl, 2), za);
        prop.h_e[1] = prop.h_g[1] + dim(at(pfl, np + 2), zb);
        let horizon = |h_e: f64, prop: &Prop| {
            (2.0 * h_e / prop.gamma_e).sqrt() * (-0.07 * (prop.delta_h / h_e.max(5.0)).sqrt()).exp()
        };
        for j in 0..2 {
            prop.d_lj[j] = horizon(prop.h_e[j], prop);
        }
        let mut q = prop.d_lj[0] + prop.d_lj[1];
        if q <= prop.d {
            q = (prop.d / q) * (prop.d / q);
            for j in 0..2 {
                prop.h_e[j] *= q;
                prop.d_lj[j] = horizon(prop.h_e[j], prop);
            }
        }
        for j in 0..2 {
            let q = (2.0 * prop.h_e[j] / prop.gamma_e).sqrt();
            prop.theta_ej[j] =
                (0.65 * prop.delta_h * (q / prop.d_lj[j] - 1.0) - 2.0 * prop.h_e[j]) / q;
        }
    } else {
        let (za, _) = zlsq1(pfl, xl[0], 0.9 * prop.d_lj[0]);
        let (_, zb) = zlsq1(pfl, prop.d - 0.9 * prop.d_lj[1], xl[1]);
        prop.h_e[0] = prop.h_g[0] + dim(at(pfl, 2), za);
        prop.h_e[1] = prop.h_g[1] + dim(at(pfl, np + 2), zb);
    }
    prop.mdp = -1;
    propv.lvar = propv.lvar.max(3);
    if mdvarx >= 0 {
        propv.mdvar = mdvarx;
        propv.lvar = propv.lvar.max(4);
    }
    if klimx > 0 {
        propv.klim = klimx;
        propv.lvar = 5;
    }
    lrprop(0.0, prop, c);
}

/// Path loss in dB (and the ITM error code) for the profile `elev` = `[np, step, z0..]`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn point_to_point(
    elev: &[f64],
    tht_m: f64,
    rht_m: f64,
    eps_dielect: f64,
    sgm_conductivity: f64,
    eno: f64,
    frq_mhz: f64,
    radio_climate: i32,
    pol: i32,
    conf: f64,
    rel: f64,
) -> (f64, i32) {
    let mut prop = Prop {
        h_g: [tht_m, rht_m],
        mdp: -1,
        ..Prop::default()
    };
    let mut propv = PropV {
        klim: radio_climate,
        lvar: 5,
        ..PropV::default()
    };
    let mut c = Ctx::default();
    let zc = qerfi(conf);
    let zr = qerfi(rel);
    let np = elev[0] as i64;
    let ja = (3.0 + 0.1 * elev[0]) as i64;
    let jb = np - ja + 6;
    let mut zsys = 0.0;
    for i in (ja - 1)..jb {
        zsys += at(elev, i);
    }
    zsys /= (jb - ja + 1) as f64;
    propv.mdvar = 12;
    qlrps(
        frq_mhz,
        zsys,
        eno,
        pol,
        eps_dielect,
        sgm_conductivity,
        &mut prop,
    );
    let klim = propv.klim;
    let mdvar = propv.mdvar;
    qlrpfl(elev, klim, mdvar, &mut prop, &mut propv, &mut c);
    let fs = 32.45 + 20.0 * frq_mhz.log10() + 20.0 * (prop.d / 1000.0).log10();
    let loss = avar(zr, 0.0, zc, &mut prop, &mut propv, &mut c.avar) + fs;
    (loss, prop.kwx)
}

/// ACRE's "Longley-Rice (ITM)" model.
#[must_use]
pub fn process(map: &Map, link: &Link) -> Signal {
    if link.frequency < 20.0 || link.frequency > 20000.0 {
        return Signal::from_dbm(-999.0);
    }
    let polarization = link.tx_antenna.polarization;
    if polarization == Polarization::Circular {
        return Signal::from_dbm(-999.0);
    }
    let (tx_gain, rx_gain) = link.gains_towards(link.rx_pos, link.tx_pos);
    let budget = f64::from(mw_to_dbm(link.power)) + f64::from(tx_gain)
        - f64::from(link.tx_antenna.internal_loss_dbm)
        + f64::from(rx_gain)
        - f64::from(link.rx_antenna.internal_loss_dbm);

    // [np, step, elevations…]; np counted the two header values too, as in the C++, and the
    // profile is padded with its last value.
    let sample_size = 7.5f32;
    let mut profile = vec![0.0, f64::from(sample_size)];
    profile.extend(
        map.terrain_profile(link.tx_pos, link.rx_pos, sample_size)
            .into_iter()
            .map(f64::from),
    );
    profile[0] = (profile.len() - 1) as f64;
    let last = *profile.last().expect("non-empty");
    profile.extend(std::iter::repeat_n(last, 10));

    let pol = match polarization {
        Polarization::Horizontal => 0,
        Polarization::Vertical => 1,
        Polarization::Circular => 2,
    };
    // ITM wants each antenna's height above the ground, transmitter first (the profile runs
    // from the transmitter). The C++ passed the positions' heights above sea level, receiver
    // first, so hills became masts. Clamped to ITM's valid range: an antenna slightly below
    // the interpolated ground would otherwise make ITM return NaN.
    let height = |p: Vec3| f64::from(p.z - map.elevation(p.x, p.y)).clamp(0.5, 3000.0);
    let (loss, _) = point_to_point(
        &profile,
        height(link.tx_pos),
        height(link.rx_pos),
        15.0,
        0.005,
        301.0,
        f64::from(link.frequency),
        map.climate,
        pol,
        0.90,
        0.90,
    );
    let dbm = (budget - loss) as f32;
    Signal {
        dbm,
        v: dbm_to_v(dbm, 50.0),
        reflections: Vec::new(),
    }
}

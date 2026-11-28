//! Compares the Rust terrain loading, antenna gains and propagation models with the original
//! C++ on a real terrain (`vn_the_bra.fakewrp`) and the shipped antennas. The golden file comes
//! from `reference/generate.sh`.

use std::sync::Arc;

use acre2_signal::{
    antenna::Antenna,
    map::Map,
    math::Vec3,
    models::{Link, itm, los, multipath::Multipath},
    wrp::Landscape,
};

fn repo(path: &str) -> String {
    format!("{}/../../../{path}", env!("CARGO_MANIFEST_DIR"))
}

fn nums(parts: &[&str]) -> Vec<f32> {
    parts.iter().map(|p| p.parse().unwrap()).collect()
}

fn v3(n: &[f32]) -> Vec3 {
    Vec3::new(n[0], n[1], n[2])
}

/// Within `tol`, or both effectively "no signal".
fn close(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol * (1.0 + b.abs()) || (a <= -900.0 && b <= -900.0)
}

#[test]
#[allow(clippy::too_many_lines)]
fn matches_cpp() {
    let golden = std::fs::read_to_string(format!(
        "{}/tests/golden/signal.txt",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let landscape =
        Landscape::parse(&std::fs::read(repo("addons/compat_sog/vn_the_bra.fakewrp")).unwrap())
            .unwrap();
    let mut map = Map::from_landscape(&landscape);
    map.climate = 5;
    let map = Arc::new(map);
    let whip = Antenna::parse(
        &std::fs::read(repo("addons/sys_antenna/binary/Harris_123cm_Whip_gain.aba")).unwrap(),
    )
    .unwrap();
    let prc343 =
        Antenna::parse(&std::fs::read(repo("addons/sys_antenna/binary/prc343_gain.aba")).unwrap())
            .unwrap();

    let mut peaks = Vec::new();
    let (mut elev_n, mut gain_n, mut case_n) = (0, 0, 0);
    let mut worst = [0f32; 3]; // los, multipath, itm (dB)
    let mut failures = Vec::new();

    for line in golden.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "map" => {
                assert_eq!(parts[1].parse::<u32>().unwrap(), map.size());
                assert!((parts[2].parse::<f32>().unwrap() - map.cell_size()).abs() < 1e-6);
                assert_eq!(parts[3], "0", "C++ load result");
            }
            "peaks" => assert_eq!(parts[1].parse::<usize>().unwrap(), map.peaks.len()),
            "peak" => peaks.push(v3(&nums(&parts[1..4]))),
            "elev" => {
                let n = nums(&parts[1..]);
                let (z, dx, dy) = map.elevation_slope(n[0], n[1]);
                let normal = map.normal(n[0], n[1]);
                assert!(
                    close(z, n[2], 1e-6),
                    "elevation at {},{}: {z} vs {}",
                    n[0],
                    n[1],
                    n[2]
                );
                assert!(
                    close(dx, n[3], 1e-5) && close(dy, n[4], 1e-5),
                    "slope: {line}"
                );
                assert!(normal.distance(v3(&n[5..8])) < 1e-5, "normal: {line}");
                elev_n += 1;
            }
            "gain" => {
                let n = nums(&parts[1..]);
                let g = whip.gain(v3(&n[0..3]), v3(&n[3..6]), n[6]);
                assert!(close(g, n[7], 1e-4), "gain: {line}: {g}");
                gain_n += 1;
            }
            "case" => {
                let uhf = parts[1] == "1";
                let n = nums(&parts[2..]);
                let antenna = if uhf { &prc343 } else { &whip };
                let link = Link {
                    tx_pos: v3(&n[0..3]),
                    tx_dir: v3(&n[3..6]),
                    rx_pos: v3(&n[6..9]),
                    rx_dir: v3(&n[9..12]),
                    tx_antenna: antenna,
                    rx_antenna: antenna,
                    frequency: n[12],
                    power: n[13],
                    scale: n[14],
                    omnidirectional: n[15] != 0.0,
                };
                let expected = &n[16..];
                let mp = Multipath::new(map.clone()).process(&link);
                let results = [
                    los::process(&map, &link),
                    mp.clone(),
                    itm::process(&map, &link),
                ];
                for (i, r) in results.iter().enumerate() {
                    let (dbm, v) = (expected[i * 2], expected[i * 2 + 1]);
                    if dbm > -900.0 {
                        worst[i] = worst[i].max((r.dbm - dbm).abs());
                    }
                    if !close(r.dbm, dbm, 1e-4) || !close(r.v, v, 1e-3) {
                        failures.push(format!(
                            "model {i}: {} dBm / {} V, C++ {dbm} / {v}: {line}",
                            r.dbm, r.v
                        ));
                    }
                }
                assert_eq!(
                    mp.reflections.len(),
                    expected[6] as usize,
                    "reflections: {line}"
                );
                case_n += 1;
            }
            other => panic!("unknown line {other}"),
        }
    }

    assert_eq!(peaks, map.peaks, "peaks");
    assert_eq!((elev_n, gain_n, case_n), (200, 100, 120));
    println!(
        "worst |Δ dBm|: los {} multipath {} itm {}",
        worst[0], worst[1], worst[2]
    );
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

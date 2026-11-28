//! Radio signal propagation (`signal:*`): the terrain, antenna gain patterns and propagation
//! models ([`acre2_signal`], ported from the C++ `acre_x64` extension).
//!
//! Long work runs off Arma's thread and answers with `ExtensionCallback`, name [`CALLBACK`]
//! and data `[index, result]`. `index` is whatever SQF passed: its slot in
//! `acre_sys_core_threadedExtCalls`, which holds the SQF callback. These commands carry their
//! own [`Context`], so they work before `init`.

use std::{
    collections::HashMap,
    sync::{
        LazyLock, Mutex, OnceLock,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
};

use acre2_signal::{Engine, Model, Request, Vec3, signal_map};
use arma_rs::{Context, Group, Value};

/// `ExtensionCallback` name for signal results.
pub const CALLBACK: &str = "YARD_SIGNAL";

static ENGINE: LazyLock<Engine> = LazyLock::new(Engine::default);

type Job = Box<dyn FnOnce() + Send>;

/// Runs `job` on the signal worker pool (one thread per core, at most 8).
fn spawn(job: impl FnOnce() + Send + 'static) {
    static POOL: OnceLock<Mutex<mpsc::Sender<Job>>> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        let rx = std::sync::Arc::new(Mutex::new(rx));
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
        for i in 0..threads {
            let rx = rx.clone();
            std::thread::Builder::new()
                .name(format!("yard-signal-{i}"))
                .spawn(move || {
                    loop {
                        let job = rx.lock().expect("signal pool").recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => break,
                        }
                    }
                })
                .expect("spawning signal worker");
        }
        Mutex::new(tx)
    });
    let _ = pool.lock().expect("signal pool").send(Box::new(job));
}

fn reply(ctx: &Context, func: &str, data: impl arma_rs::IntoArma) {
    if ctx.callback_data(CALLBACK, func, data).is_err() {
        tracing::warn!(func, "signal callback dropped");
    }
}

pub fn group() -> Group {
    Group::new()
        .command("load_map", load_map)
        .command("antenna", antenna)
        .command("process", process)
        .command("map", map)
        .command("map_progress", map_progress)
        .command("map_sample", map_sample)
}

/// `["signal:load_map", [index, wrpPath, climate]]`: loads the terrain (a `.wrp` from the game's
/// PBOs, or a compat `.fakewrp`) in the background. Callback `load_map` with
/// `[index, result]`: 0 loaded, -1 loaded flat (elevations unreadable), -2 failed.
fn load_map(ctx: Context, index: u32, wrp: String, climate: i32) {
    std::thread::spawn(move || {
        let result = ENGINE.load_map(&wrp, climate) as i32;
        reply(&ctx, "load_map", (index, result));
    });
}

/// `["signal:antenna", [name, gainFile]]` → `true` once the antenna's gain pattern is loaded
/// (from the game's PBOs).
fn antenna(name: String, path: String) -> bool {
    ENGINE.load_antenna(&name, &path)
}

/// `["signal:process", [index, model, id, txPos, txDir, txAntenna, rxPos, rxDir, rxAntenna,
/// frequencyMHz, powerMW, terrainScaling, omnidirectional]]`: signal at the receiver antenna.
/// Callback `process` with `[index, [id, dBm, V]]`, or `[index, []]` when an antenna isn't
/// loaded.
fn process(
    ctx: Context,
    index: u32,
    model: i32,
    id: String,
    tx_pos: [f32; 3],
    tx_dir: [f32; 3],
    tx_antenna: String,
    rx_pos: [f32; 3],
    rx_dir: [f32; 3],
    rx_antenna: String,
    frequency: f32,
    power: f32,
    scale: f32,
    omnidirectional: bool,
) {
    let request = Request {
        model: Model::from_index(model),
        tx_pos: tx_pos.into(),
        tx_dir: tx_dir.into(),
        tx_antenna,
        rx_pos: rx_pos.into(),
        rx_dir: rx_dir.into(),
        rx_antenna,
        frequency,
        power,
        scale,
        omnidirectional,
    };
    spawn(move || {
        let result = match ENGINE.process(&request) {
            Some(s) => vec![
                Value::String(id),
                Value::Number(f64::from(s.dbm)),
                Value::Number(f64::from(s.v)),
            ],
            None => Vec::new(),
        };
        reply(&ctx, "process", (index, result));
    });
}

/// Signal map progress and finished areas.
static PROGRESS: AtomicU32 = AtomicU32::new(0);
static TOTAL: AtomicU32 = AtomicU32::new(0);
static AREAS: LazyLock<Mutex<HashMap<String, signal_map::Result>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// `["signal:map", [index, id, sampleSize, areaStart, areaEnd, txPos, txDir, txHeight,
/// rxHeight, txAntenna, rxAntenna, frequencyMHz, powerMW, lowerSensitivity,
/// upperSensitivity, omnidirectional]]`: the signal map debug tool. Computes the area in the
/// background and writes `userconfig\<id>.paa` (in the game folder). Callback `map` with
/// `[index, ok]`.
fn map(
    ctx: Context,
    index: u32,
    id: String,
    sample_size: f32,
    start: [f32; 3],
    end: [f32; 3],
    tx_pos: [f32; 3],
    tx_dir: [f32; 3],
    tx_height: f32,
    rx_height: f32,
    tx_antenna: String,
    rx_antenna: String,
    frequency: f32,
    power: f32,
    lower_sensitivity: f32,
    upper_sensitivity: f32,
    omnidirectional: bool,
) {
    let area = signal_map::Area {
        start: start.into(),
        end: end.into(),
        sample_size,
        tx_pos: tx_pos.into(),
        tx_dir: tx_dir.into(),
        tx_height,
        rx_height,
        frequency,
        power,
        lower_sensitivity,
        upper_sensitivity,
        omnidirectional,
    };
    std::thread::spawn(move || {
        let Some(result) = ENGINE.signal_map(&area, &tx_antenna, &rx_antenna, &PROGRESS, &TOTAL)
        else {
            reply(&ctx, "map", (index, false));
            return;
        };
        let texture = signal_map::texture(&result, &area);
        let dir = std::path::Path::new("userconfig");
        let written = std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(dir.join(format!("{id}.paa")), texture));
        if let Err(e) = &written {
            tracing::error!(error = %e, "writing the signal map texture");
        }
        AREAS.lock().expect("signal areas").insert(id, result);
        reply(&ctx, "map", (index, written.is_ok()));
    });
}

/// `["signal:map_progress", []]` → `[done, total]` samples of the area being computed.
fn map_progress() -> (u32, u32) {
    (
        PROGRESS.load(Ordering::Relaxed),
        TOTAL.load(Ordering::Relaxed),
    )
}

fn vec3(v: Vec3) -> Value {
    Value::Array(vec![
        Value::Number(f64::from(v.x)),
        Value::Number(f64::from(v.y)),
        Value::Number(f64::from(v.z)),
    ])
}

/// `["signal:map_sample", [id, x, y]]` → `[txPos, rxPos, dBm, [[point, normal, phase,
/// coefficient, dBm], …]]` for one sample of a finished area, or `[]`.
fn map_sample(id: String, x: u32, y: u32) -> Vec<Value> {
    let areas = AREAS.lock().expect("signal areas");
    let Some(sample) = areas.get(&id).and_then(|a| a.sample(x, y)) else {
        return Vec::new();
    };
    let reflections = sample
        .signal
        .reflections
        .iter()
        .map(|r| {
            Value::Array(vec![
                vec3(r.point),
                vec3(r.normal),
                Value::Number(f64::from(r.phase)),
                Value::Number(f64::from(r.reflect_coefficient)),
                Value::Number(f64::from(r.path_budget_dbm)),
            ])
        })
        .collect();
    vec![
        vec3(sample.tx_pos),
        vec3(sample.rx_pos),
        Value::Number(f64::from(sample.signal.dbm)),
        Value::Array(reflections),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use arma_rs::testing::Result as CbResult;
    use std::time::Duration;

    /// Waits for a `YARD_SIGNAL` callback `func` and returns its data.
    fn wait(ext: &arma_rs::testing::Extension, func: &str) -> String {
        let r = ext.callback_handler(
            |name, f, data| {
                if name == CALLBACK && f == func {
                    CbResult::Ok(data.map(|d| d.to_string()).unwrap_or_default())
                } else {
                    CbResult::<String, ()>::Continue
                }
            },
            Duration::from_secs(30),
        );
        match r {
            CbResult::Ok(d) => d,
            _ => panic!("no {func} callback"),
        }
    }

    /// A terrain and an antenna in a PBO this process holds open, found the way Arma's are.
    #[cfg(target_os = "linux")]
    #[test]
    fn terrain_antenna_and_signal_from_open_pbo() {
        use acre2_signal::{pbo, wrp};

        let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
        let size = 256u32;
        let landscape = wrp::Landscape {
            filetype: *b"ACRE",
            version: 25,
            layer_size: (size, size),
            map_size: (size, size),
            cell_size: 10.0,
            grid_size: 10.0,
            elevations: (0..size * size)
                .map(|i| 20.0 + 15.0 * ((i % size) as f32 * 0.07).sin())
                .collect(),
            peaks: vec![],
            result: wrp::LoadResult::Success,
        };
        let aba = std::fs::read(format!(
            "{repo}/addons/sys_antenna/binary/Harris_123cm_Whip_gain.aba"
        ))
        .unwrap();
        let fakewrp = landscape.to_fakewrp();
        let dir = std::env::temp_dir().join(format!("yard_signal_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_terrain.pbo");
        std::fs::write(
            &path,
            pbo::write(
                "z\\test\\addons\\terrain",
                &[("test.fakewrp", &fakewrp), ("whip.aba", &aba)],
            ),
        )
        .unwrap();
        let _held = std::fs::File::open(&path).unwrap();

        let ext = ext().testing();
        let (out, _) = ext.call(
            "signal:antenna",
            Some(vec![
                "ACRE_WHIP".into(),
                r"\z\test\addons\terrain\whip".into(),
            ]),
        );
        assert_eq!(out, "true");

        // Before a terrain loads, every model falls back to arcade.
        let pos = |x: f32, y: f32, z: f32| format!("[{x},{y},{z}]");
        let process = |index: u32, model: i32| {
            vec![
                index.to_string(),
                model.to_string(),
                "\"tx_rx\"".into(),
                pos(500.0, 500.0, 40.0),
                pos(1.0, 0.0, 0.0),
                "\"ACRE_WHIP\"".into(),
                pos(2000.0, 1500.0, 40.0),
                pos(0.0, 1.0, 0.0),
                "\"ACRE_WHIP\"".into(),
                "150".into(),
                "5000".into(),
                "1".into(),
                "false".into(),
            ]
        };
        let (_, code) = ext.call("signal:process", Some(process(7, 2)));
        assert_eq!(code, 0);
        let arcade = wait(&ext, "process");
        assert!(arcade.starts_with("[7,[\"tx_rx\","), "{arcade}");

        let (_, code) = ext.call(
            "signal:load_map",
            Some(vec![
                "3".into(),
                r"\z\test\addons\terrain\test.fakewrp".into(),
                "5".into(),
            ]),
        );
        assert_eq!(code, 0);
        assert_eq!(wait(&ext, "load_map"), "[3,0]");

        for model in [1, 2, 3] {
            let (_, code) = ext.call("signal:process", Some(process(model as u32, model)));
            assert_eq!(code, 0);
            let data = wait(&ext, "process");
            assert!(
                data.starts_with(&format!("[{model},[\"tx_rx\",-")),
                "{data}"
            );
        }

        // An antenna that was never loaded: an empty result.
        let mut args = process(9, 2);
        args[5] = "\"ACRE_NOPE\"".into();
        let (_, code) = ext.call("signal:process", Some(args));
        assert_eq!(code, 0);
        assert_eq!(wait(&ext, "process"), "[9,[]]");
        std::fs::remove_dir_all(dir).ok();
    }

    fn ext() -> arma_rs::Extension {
        arma_rs::Extension::build()
            .group("signal", group())
            .finish()
    }

    #[test]
    fn antenna_missing_and_progress() {
        let ext = ext().testing();
        let (out, code) = ext.call(
            "signal:antenna",
            Some(vec![
                "ACRE_TEST".to_string(),
                r"\nope\missing.aba".to_string(),
            ]),
        );
        assert_eq!(code, 0);
        assert_eq!(out, "false");
        let (out, code) = ext.call("signal:map_progress", None);
        assert_eq!((out.as_str(), code), ("[0,0]", 0));
        let (out, _) = ext.call(
            "signal:map_sample",
            Some(vec!["nope".into(), "0".into(), "0".into()]),
        );
        assert_eq!(out, "[]");
    }
}

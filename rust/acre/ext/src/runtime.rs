//! The extension's tokio runtime and logging.

use std::sync::{Mutex, OnceLock};

use tokio::runtime::Runtime;

pub fn get() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("acre2-net")
            .enable_all()
            .build()
            .expect("building tokio runtime")
    })
}

/// Reads `YARD_<suffix>`, falling back to the older `ACRE2_<suffix>`. Returns the variable's name
/// with its value.
pub fn env_var(suffix: &str) -> Option<(String, String)> {
    ["YARD_", "ACRE2_"].into_iter().find_map(|prefix| {
        let name = format!("{prefix}{suffix}");
        std::env::var(&name).ok().map(|value| (name, value))
    })
}

/// Logs to `yard_ext.log` next to the Arma profile (`%LOCALAPPDATA%\Arma 3`), or the working
/// directory. `YARD_LOG` (or `ACRE2_LOG`) sets the filter (default `info`).
pub fn init_logging() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = std::env::var_os("LOCALAPPDATA")
            .map(|d| std::path::PathBuf::from(d).join("Arma 3"))
            .filter(|d| d.is_dir())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let Ok(file) = std::fs::File::create(dir.join("yard_ext.log")) else {
            return;
        };
        let filter = env_var("LOG")
            .and_then(|(_, f)| tracing_subscriber::EnvFilter::try_new(f).ok())
            .unwrap_or_else(|| tracing_subscriber::EnvFilter::new("info"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .try_init();
    });
}

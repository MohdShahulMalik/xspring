//! Logging setup.
//!
//! Two writers are installed when logging is switched on:
//!
//! * **stderr**, filtered at the level the user asked for on the command line
//!   (`-v` → warnings, `-vv` → info, `-vvv` → debug, `-vvvv` → trace);
//! * **a daily rotating file**, filtered at `info` and above so the file always
//!   records what the run did, where the time went, whether the metadata came
//!   from the cache, how long the download took.
//!
//! The file is only created when the user asked for more than errors, so a
//! plain `xspring list --java` still leaves no trace in the working directory.

use std::path::PathBuf;

use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// Environment variable that overrides the maximum level written to the file.
const LOG_LEVEL_ENV: &str = "XSPRING_LOG";

/// Environment variable that overrides the directory the log file lives in.
const LOG_DIR_ENV: &str = "XSPRING_LOG_DIR";

/// Default directory for the rotating log file, relative to the current
/// working directory.
const DEFAULT_LOG_DIR: &str = "logs";

/// Installs the global tracing subscriber.
///
/// Does nothing unless the user asked for more than errors, because errors
/// are already reported on stderr by the `anyhow` error returned from `main`.
pub fn init(level: LevelFilter) {
    if level <= LevelFilter::ERROR {
        return;
    }

    let dir = log_dir();
    if let Err(err) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "xspring: could not create the log directory {}: {err}",
            dir.display()
        );
        return;
    }

    let file_level = file_level(level);
    let appender = rolling::daily(&dir, "xspring.log");

    let terminal = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_filter(filter_fn(move |metadata| *metadata.level() <= level));
    let file = fmt::layer()
        .with_writer(appender)
        .with_ansi(false)
        .with_filter(filter_fn(move |metadata| *metadata.level() <= file_level));

    // The registry level is the most verbose of the two writers, so neither
    // of them is silently starved by the other.
    let registry_level = std::cmp::max(level, file_level);

    tracing_subscriber::registry()
        .with(registry_level)
        .with(terminal)
        .with(file)
        .init();

    eprintln!("xspring: logging to {}", dir.display());
    info!("xspring: logging to {}", dir.display());
}

/// The directory the rotating log file is written to.
fn log_dir() -> PathBuf {
    match crate::util::env_non_empty(LOG_DIR_ENV) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(DEFAULT_LOG_DIR),
    }
}

/// The maximum level written to the log file.
///
/// Defaults to `info` so the file always explains the run, even when only
/// warnings were requested on the terminal. Override with `XSPRING_LOG`.
fn file_level(level: LevelFilter) -> LevelFilter {
    if let Some(value) = crate::util::env_non_empty(LOG_LEVEL_ENV) {
        if let Some(parsed) = parse_level(&value) {
            return parsed;
        }
        eprintln!(
            "xspring: ignoring unknown {} value {:?}",
            LOG_LEVEL_ENV, value
        );
    }

    std::cmp::max(level, LevelFilter::INFO)
}

/// Parses a level name such as `info` or `TRACE`.
fn parse_level(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" | "none" => Some(LevelFilter::OFF),
        "error" => Some(LevelFilter::ERROR),
        "warn" | "warning" => Some(LevelFilter::WARN),
        "info" => Some(LevelFilter::INFO),
        "debug" => Some(LevelFilter::DEBUG),
        "trace" => Some(LevelFilter::TRACE),
        _ => None,
    }
}

//! This module handles setting up logging with `tracing`.

use tracing_appender::{non_blocking, non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;

/// The directory for the server's log files.
const LOG_DIR: &str = concat!(env!("DATA_DIR"), "/scanner_logs");

/// The common prefix for the server's log files.
const LOG_PREFIX: &str = "scanner_server.log";

/// Initialise a subscriber for tracing to log to `stdout` and a file.
pub fn init_tracing() -> WorkerGuard {
    let (appender, guard) = non_blocking(rolling::never(LOG_DIR, LOG_PREFIX));

    let subscriber = tracing_subscriber::registry()
        .with(
            Layer::new()
                .with_writer(appender)
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy(""),
                ),
        )
        .with(
            Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy(),
                ),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect_or_log("Setting the global default for tracing should be okay");

    guard
}

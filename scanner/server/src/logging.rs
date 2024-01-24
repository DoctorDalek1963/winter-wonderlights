//! This module handles setting up logging with `tracing`.

use tracing_appender::{non_blocking, non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;

/// Initialise a subscriber for tracing to log to `stdout` and a file.
pub fn init_tracing() -> WorkerGuard {
    #[allow(
        clippy::expect_used,
        reason = "we can't call expect_or_log before we've initted tracing"
    )]
    let (appender, guard) = non_blocking(rolling::never(
        format!(
            "{}/scanner_logs",
            std::env::var("DATA_DIR").expect("DATA_DIR must be defined")
        ),
        "scanner_server.log",
    ));

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

//! This binary crate runs the server for `Winter WonderLights`.

#![feature(lint_reasons)]

mod drivers;
mod run_effect;
mod run_server;

use chrono::naive::NaiveDate;
use color_eyre::Result;
use regex::Regex;
use std::{
    fs::{self, DirEntry},
    ops::Deref,
    process::Command,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{signal, sync::oneshot};
use tracing::{debug, error, info, instrument, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;
use ww_effects::traits::get_config_filename;
use ww_shared::ClientState;

/// The version of this crate.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The directory for the server's log files.
const LOG_DIR: &str = concat!(env!("DATA_DIR"), "/logs");

/// The common prefix for the server's log files.
const LOG_PREFIX: &str = "server.log";

/// The filename for the server state config.
const SERVER_STATE_FILENAME: &str = "server_state.ron";

lazy_static::lazy_static! {
    /// A RegEx to match against the filenames of the server's log files and extract the date parts.
    static ref SERVER_LOG_REGEX: Regex = Regex::new(&{
        let mut s = regex::escape(LOG_PREFIX);
        s.push_str(r"\.(\d{4})-(\d{2})-(\d{2})$");
        s
    }).expect("Regex should compile successfully");
}

/// A simple wrapper struct to hold the client state.
#[derive(Clone, Debug)]
pub struct WrappedClientState(Arc<RwLock<ClientState>>);

impl WrappedClientState {
    /// Initialise the server state.
    fn new() -> Self {
        Self(Arc::new(RwLock::new(ClientState::from_file(
            SERVER_STATE_FILENAME,
        ))))
    }

    /// Save the config of the client state.
    #[instrument(skip_all)]
    fn save_config(&self) {
        if let Some(config) = &self
            .read()
            .expect_or_log("Should be able to read client state")
            .effect_config
        {
            info!(?config, "Saving config to file");
            config.save_to_file(&get_config_filename(config.effect_name()));
        } else {
            debug!("Tried to save config but it's None, so skipping");
        }
    }
}

impl Deref for WrappedClientState {
    type Target = RwLock<ClientState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for WrappedClientState {
    fn drop(&mut self) {
        self.read()
            .unwrap_or_log()
            .save_to_file(SERVER_STATE_FILENAME);
    }
}

/// Initialise a subscriber for tracing to log to `stdout` and a file.
fn init_tracing() -> WorkerGuard {
    let (appender, guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily(LOG_DIR, LOG_PREFIX));

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

/// Compress all log files older than the given number of days using `gzip`.
#[instrument]
fn zip_old_log_files(days: u64) {
    let today = chrono::offset::Local::now().date_naive();

    // Look through everything in the log files folder and filter it down to just the log files and
    // parse their dates, and then filter down to only the log files which are older than the given
    // number of days
    let log_files: Vec<_> = fs::read_dir(LOG_DIR)
        .expect_or_log(&format!("Should be able to read entries in {LOG_DIR}"))
        .filter_map(|file_result| match file_result {
            Ok(dir_entry)
                if dir_entry
                    .file_type()
                    .is_ok_and(|filetype| filetype.is_file()) =>
            {
                dir_entry
                    .file_name()
                    .to_str()
                    .and_then(|name| -> Option<(DirEntry, NaiveDate)> {
                        let captures = SERVER_LOG_REGEX.captures(name)?;

                        let year = captures.get(1)?.as_str().parse().ok()?;
                        let month = captures.get(2)?.as_str().parse().ok()?;
                        let day = captures.get(3)?.as_str().parse().ok()?;

                        Some((dir_entry, NaiveDate::from_ymd_opt(year, month, day)?))
                    })
            }
            _ => None,
        })
        .filter_map(|(file, date)| {
            if (today - date).num_days() > days as i64 {
                Some(file.path())
            } else {
                None
            }
        })
        .collect();

    if !log_files.is_empty() {
        let gzip_command = Command::new("gzip")
            .args(log_files)
            .spawn()
            .expect_or_log("Should be able to run `gzip` on old log files");

        if let Some(stderr) = gzip_command.stderr {
            error!(?stderr, "gzip command failed when zipping old log files");
        }
    }
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // _guard gets dropped at the end of main so that the logs get flushed to the file
    let _guard = init_tracing();

    let client_state = WrappedClientState::new();
    let (kill_run_effect_thread_tx, kill_run_effect_thread_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            zip_old_log_files(3);

            // Sleep for 1 day
            tokio::time::sleep(Duration::from_secs(60 * 60 * 24)).await;
        }
    });

    let ret_val = tokio::select! {
        biased;

        _ = signal::ctrl_c() => {
            info!("Recieved ^C. Now terminating");
            kill_run_effect_thread_tx
                .send(())
                .expect_or_log("Should be able to send () to run-effect thread to kill it");
            Ok(())
        }
        ret = self::run_server::run_server(client_state.clone(), kill_run_effect_thread_rx) => {
            ret
        }
    };

    self::run_server::terminate_all_client_connections();
    client_state.save_config();

    ret_val
}

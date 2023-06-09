//! This binary crate runs the server for Winter WonderLights.

mod drivers;
mod run_effect;
mod run_server;

use color_eyre::Result;
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};
use tokio::{signal, sync::oneshot};
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;
use ww_effects::traits::get_config_filename;
use ww_shared::ClientState;

/// The version of this crate.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The filename for the server state config.
const SERVER_STATE_FILENAME: &str = "server_state.ron";

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
        &*self.0
    }
}

impl Drop for WrappedClientState {
    fn drop(&mut self) {
        self.read()
            .unwrap_or_log()
            .save_to_file(SERVER_STATE_FILENAME)
    }
}

/// Initialise a subscriber for tracing to log to `stdout` and a file.
fn init_tracing() {
    let appender =
        tracing_appender::rolling::daily(concat!(env!("DATA_DIR"), "/logs"), "server.log");

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
                .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into())),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect_or_log("Setting the global default for tracing should be okay");
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_tracing();

    let client_state = WrappedClientState::new();
    let (kill_run_effect_thread_tx, kill_run_effect_thread_rx) = oneshot::channel();

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

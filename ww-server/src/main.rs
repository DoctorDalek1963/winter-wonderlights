//! This binary crate runs the server for `Winter WonderLights`.

#![feature(lint_reasons)]

mod drivers;
mod logging;
mod run_effect;
mod run_server;

use color_eyre::Result;
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{signal, sync::oneshot};
use tracing::{debug, info, instrument, warn};
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
        &self.0
    }
}

impl Drop for WrappedClientState {
    fn drop(&mut self) {
        self.save_config();
        self.read()
            .unwrap_or_log()
            .save_to_file(SERVER_STATE_FILENAME);
    }
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // _guard gets dropped at the end of main so that the logs get flushed to the file
    let _guard = self::logging::init_tracing();

    let client_state = WrappedClientState::new();
    let (kill_run_effect_thread_tx, kill_run_effect_thread_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            self::logging::zip_log_files_older_than_hours(3).await;
            self::logging::zip_log_files_older_than_days(2).await;

            // Sleep for 1 hour
            tokio::time::sleep(Duration::from_secs(60 * 60)).await;
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

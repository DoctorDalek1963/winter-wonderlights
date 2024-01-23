//! This binary crate runs the server for `Winter WonderLights`.

#![feature(lint_reasons)]

mod drivers;
mod gift;
mod logging;
mod run_server;
mod scan_manager;
mod state;

use color_eyre::Result;
use std::sync::atomic::AtomicBool;
use tokio::{signal, sync::oneshot};
use tracing::{info, instrument, warn};
use tracing_unwrap::ResultExt;

/// The version of this crate.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Have we finished channing the tree yet?
pub(crate) static FINISHED_SCANNING: AtomicBool = AtomicBool::new(false);

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // _guard gets dropped at the end of main so that the logs get flushed to the file
    let _guard = self::logging::init_tracing();

    let (kill_tx, kill_rx) = oneshot::channel();

    let ret_val = tokio::select! {
        biased;

        _ = signal::ctrl_c() => {
            info!("Recieved ^C. Now terminating");
            kill_tx
                .send(())
                .expect_or_log("Should be able to send () to run-server thread to kill it");
            Ok(())
        }
        ret = self::run_server::run_server(kill_rx) => {
            ret
        }
    };

    self::run_server::terminate_all_client_connections();

    ret_val
}

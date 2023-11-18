//! This module manages the scan and acts as the internal server, coordinating the camera and
//! controller clients.

use lazy_static::lazy_static;
use tokio::sync::{broadcast, oneshot};
use tracing::{info, instrument, trace};
use tracing_unwrap::ResultExt;
use ww_driver_trait::Driver;

lazy_static! {
    /// Send messages to the scan manager.
    pub static ref SEND_MESSAGE_TO_SCAN_MANAGER: broadcast::Sender<ScanManagerMsg> = broadcast::channel(10).0;
}

/// Possible messages to send to the scan manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(dead_code, reason = "message sending not implemented yet")]
pub enum ScanManagerMsg {
    /// Start to take photos.
    StartTakingPhotos,
}

/// Run the scan manager.
#[instrument(skip_all)]
pub fn run_scan_manager(kill_rx: oneshot::Receiver<()>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap_or_log();
    let local = tokio::task::LocalSet::new();

    // Safety: This function gets run once in a background thread for the duration of the server,
    // so this call to `init()` only happens once and is thus safe.
    let _driver = unsafe { crate::drivers::DriverWrapper::init() };

    let mut thread_message_rx = SEND_MESSAGE_TO_SCAN_MANAGER.subscribe();

    info!("Beginning scan manager loop");

    let recv_msgs_and_manage_scan = async move {
        loop {
            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = thread_message_rx.recv() => {
                    trace!(?msg, "Recieved ScanManagerMsg");

                    match msg.expect_or_log("There should not be an error in receiving a ScanManagerMsg") {
                        ScanManagerMsg::StartTakingPhotos => todo!("Handle StartTakingPhotos"),
                    }
                }

                _ = async { loop {
                    info!("Scan manager is running");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }} => {}
            }
        }
    };

    local.block_on(&runtime, async move {
        tokio::select! {
            biased;

            // If we get told to kill this thread, then immediately return. This manual return
            // ensures that `driver` gets dropped, so that its drop impl gets correctly called
            _ = kill_rx => {
                #[allow(
                    clippy::needless_return,
                    reason = "this explicit return is clearer than an implicit one"
                )]
                return;
            }

            _ = recv_msgs_and_manage_scan => {}
        }
    });
}

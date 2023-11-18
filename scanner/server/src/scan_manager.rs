//! This module manages the scan and acts as the internal server, coordinating the camera and
//! controller clients.

use crate::run_server::{CAMERA_SEND, CONTROLLER_SEND};
use lazy_static::lazy_static;
use std::time::Duration;
use tokio::{
    sync::{broadcast, oneshot},
    time::sleep,
};
use tracing::{info, instrument, trace};
use tracing_unwrap::ResultExt;
use ww_driver_trait::Driver;
use ww_scanner_shared::{GenericServerToClientMsg, ServerToCameraMsg, ServerToControllerMsg};

lazy_static! {
    /// Send messages to the scan manager.
    pub static ref SEND_MESSAGE_TO_SCAN_MANAGER: broadcast::Sender<ScanManagerMsg> = broadcast::channel(10).0;
}

/// Possible messages to send to the scan manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanManagerMsg {
    /// The camera has connected.
    CameraConnected,

    /// The controller has connected.
    ControllerConnected,
    // Start to take photos. StartTakingPhotos,
}

/// The state of the scan manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanManagerState {
    /// We're currently waiting for both clients to connect.
    WaitingForConnections {
        /// Is the camera connected?
        camera_connected: bool,

        /// Is the controller connected?
        controller_connected: bool,
    },

    /// Both clients are connected and we're waiting to scan.
    WaitingToScan,
}

impl Default for ScanManagerState {
    fn default() -> Self {
        Self::WaitingForConnections {
            camera_connected: false,
            controller_connected: false,
        }
    }
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
    let mut state = ScanManagerState::default();

    info!("Beginning scan manager loop");

    let recv_msgs_and_manage_scan = async move {
        loop {
            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = thread_message_rx.recv() => {
                    trace!(?msg, "Recieved ScanManagerMsg");

                    match msg.expect_or_log("There should not be an error in receiving a ScanManagerMsg") {
                        ScanManagerMsg::CameraConnected => {
                            if let ScanManagerState::WaitingForConnections { camera_connected, .. } = &mut state {
                                *camera_connected = true;
                            }
                        }
                        ScanManagerMsg::ControllerConnected => {
                            if let ScanManagerState::WaitingForConnections { controller_connected, .. } = &mut state {
                                *controller_connected = true;
                            }
                        }
                        //ScanManagerMsg::StartTakingPhotos => todo!("Handle StartTakingPhotos"),
                    };
                }

                _ = async { loop {
                    if let ScanManagerState::WaitingForConnections {
                        camera_connected: true,
                        controller_connected: true,
                    } = state
                    {
                        info!("Camera and controller both connected; sending ServerReady");

                        sleep(Duration::from_millis(250)).await;
                        CAMERA_SEND
                            .send(
                                bincode::serialize(&ServerToCameraMsg::Generic(
                                    GenericServerToClientMsg::ServerReady,
                                ))
                                .expect_or_log("Should be able to serialize ServerReady"),
                            )
                            .expect_or_log("Should be able to send messge down CAMERA_SEND");
                        CONTROLLER_SEND
                            .send(
                                bincode::serialize(&ServerToControllerMsg::Generic(
                                    GenericServerToClientMsg::ServerReady,
                                ))
                                .expect_or_log("Should be able to serialize ServerReady"),
                            )
                            .expect_or_log("Should be able to send messge down CONTROLLER_SEND");

                        state = ScanManagerState::WaitingToScan;
                    }

                    sleep(Duration::from_millis(50)).await;
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

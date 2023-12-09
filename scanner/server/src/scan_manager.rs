//! This module manages the scan and acts as the internal server, coordinating the camera and
//! controller clients.

use crate::run_server::{CAMERA_SEND, CONTROLLER_SEND};
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use tokio::{
    sync::{broadcast, oneshot},
    time::sleep,
};
use tracing::{debug, info, instrument, trace};
use tracing_unwrap::ResultExt;
use ww_driver_trait::{Driver, LIGHTS_NUM};
use ww_frame::FrameType;
use ww_scanner_shared::{
    CompassDirection, GenericServerToClientMsg, ServerToCameraMsg, ServerToControllerMsg,
};

lazy_static! {
    /// Send messages to the scan manager.
    pub static ref SEND_MESSAGE_TO_SCAN_MANAGER: broadcast::Sender<ScanManagerMsg> = broadcast::channel(10).0;
}

/// The current index that's being scanned.
static CURRENT_IDX: AtomicU32 = AtomicU32::new(0);

/// Possible messages to send to the scan manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanManagerMsg {
    /// The camera has connected.
    CameraConnected,

    /// The camera has disconnected.
    CameraDisconnected,

    /// The controller has connected.
    ControllerConnected,

    /// The controller has disconnected.
    ControllerDisconnected,

    /// Start to take photos.
    StartTakingPhotos {
        camera_alignment: CompassDirection,
        pause_time_ms: u16,
    },

    /// We've received photo data from the camera.
    ReceivedPhoto {
        light_idx: u32,
        brightest_pixel_pos: (u32, u32),
    },
}

/// The state of the connections.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ConnectedState {
    /// Is the camera connected?
    camera: bool,

    /// Is the camera connected?
    controller: bool,
}

impl Default for ConnectedState {
    fn default() -> Self {
        Self {
            camera: false,
            controller: false,
        }
    }
}

/// The state of the scan manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanManagerState {
    /// We're currently waiting for both clients to connect.
    WaitingForConnections,

    /// Both clients are connected and we're waiting to scan.
    WaitingToScan,

    /// We're ready for the camera to take a photo.
    ReadyToTakePhoto,

    /// We're waiting to receive photo data from the camera.
    WaitingForPhoto,
}

impl Default for ScanManagerState {
    fn default() -> Self {
        Self::WaitingForConnections
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
    let mut driver = unsafe { crate::drivers::DriverWrapper::init() };
    let mut driver_raw_data = vec![[0; 3]; LIGHTS_NUM];

    let mut thread_message_rx = SEND_MESSAGE_TO_SCAN_MANAGER.subscribe();
    let mut connected_state = ConnectedState::default();
    let mut state = ScanManagerState::default();
    let mut current_camera_alignment = CompassDirection::South;
    let mut pause_time = 0;
    let mut photo_map: HashMap<CompassDirection, Vec<(u32, u32)>> = HashMap::new();

    info!("Beginning scan manager loop");

    macro_rules! respond_to_msg {
        ($msg:ident) => {
            trace!(?$msg, "Recieved ScanManagerMsg");

            match $msg.expect_or_log("There should not be an error in receiving a ScanManagerMsg") {
                ScanManagerMsg::CameraConnected => {
                    connected_state.camera = true;
                }
                ScanManagerMsg::CameraDisconnected => {
                    connected_state.camera = false;
                    state = ScanManagerState::WaitingForConnections;

                    // We can fail to send if there's no controller connected
                    let _ = CONTROLLER_SEND.send(
                        bincode::serialize(&ServerToControllerMsg::Generic(
                            GenericServerToClientMsg::ServerNotReady,
                        ))
                        .expect_or_log("Should be able to serialize ServerNotReady"),
                    );
                }
                ScanManagerMsg::ControllerConnected => {
                    connected_state.controller = true;
                }
                ScanManagerMsg::ControllerDisconnected => {
                    connected_state.controller = false;
                    state = ScanManagerState::WaitingForConnections;

                    // We can fail to send if there's no camera connected
                    let _ = CAMERA_SEND.send(
                        bincode::serialize(&ServerToCameraMsg::Generic(
                            GenericServerToClientMsg::ServerNotReady,
                        ))
                        .expect_or_log("Should be able to serialize ServerNotReady"),
                    );
                }
                ScanManagerMsg::StartTakingPhotos { camera_alignment, pause_time_ms } => {
                    CURRENT_IDX.store(0, Ordering::Relaxed);
                    current_camera_alignment = camera_alignment;
                    pause_time = pause_time_ms;
                    photo_map.entry(camera_alignment).and_modify(|list| list.clear());
                    state = ScanManagerState::ReadyToTakePhoto;
                }
                ScanManagerMsg::ReceivedPhoto {
                    light_idx,
                    brightest_pixel_pos,
                } => {
                    debug_assert_eq!(
                        light_idx,
                        CURRENT_IDX.load(Ordering::Relaxed) - 1,
                        "The camera and server should be in sync with the light indices"
                    );

                    info!(?light_idx, "Received photo from camera");

                    photo_map
                        .entry(current_camera_alignment)
                        .and_modify(|list| {
                            debug_assert_eq!(
                                list.len(),
                                light_idx as usize,
                                concat!(
                                    "The light_idx should be in sync with the length ",
                                    "of the corresponding vec in photo_map"
                                )
                            );
                            list.push(brightest_pixel_pos);
                        })
                        .or_insert_with(|| {
                            debug_assert_eq!(
                                light_idx,
                                0,
                                "If we're inserting a new vec into the map, the light_idx should be 0"
                            );
                            vec![brightest_pixel_pos]
                        });

                    state = ScanManagerState::ReadyToTakePhoto;
                }
            };
        };
    }

    let recv_msgs_and_manage_scan = async move {
        loop {
            let react_to_state_changes_in_loop = async {
                loop {
                    match state {
                        ScanManagerState::WaitingForConnections
                            if connected_state.camera && connected_state.controller =>
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
                                .expect_or_log(
                                    "Should be able to send messge down CONTROLLER_SEND",
                                );

                            state = ScanManagerState::WaitingToScan;
                            driver.display_frame(FrameType::RawData(vec![[255; 3]; LIGHTS_NUM]));
                        }
                        ScanManagerState::ReadyToTakePhoto => {
                            let light_idx = CURRENT_IDX.fetch_add(1, Ordering::Relaxed);
                            if light_idx as usize >= LIGHTS_NUM {
                                info!(
                                    ?current_camera_alignment,
                                    "Finished scanning from this angle"
                                );
                                debug!(?photo_map);

                                state = ScanManagerState::WaitingToScan;
                                CONTROLLER_SEND
                                    .send(
                                        bincode::serialize(
                                            &ServerToControllerMsg::PhotoSequenceDone,
                                        )
                                        .expect_or_log(
                                            "Should be able to serialize PhotoSequenceDone",
                                        ),
                                    )
                                    .expect_or_log(
                                        "Should be able to send messge down CONTROLLER_SEND",
                                    );

                                continue;
                            }

                            info!(?light_idx, "Ready to take photo");

                            driver_raw_data[light_idx as usize] = [255; 3];
                            driver.display_frame(FrameType::RawData(driver_raw_data.clone()));
                            driver_raw_data[light_idx as usize] = [0; 3];

                            sleep(Duration::from_millis(pause_time as u64)).await;
                            CAMERA_SEND
                                .send(
                                    bincode::serialize(&ServerToCameraMsg::TakePhoto { light_idx })
                                        .expect_or_log("Should be able to serialize TakePhoto"),
                                )
                                .expect_or_log("Should be able to send messge down CAMERA_SEND");
                            state = ScanManagerState::WaitingForPhoto;
                        }
                        ScanManagerState::WaitingForConnections
                        | ScanManagerState::WaitingToScan
                        | ScanManagerState::WaitingForPhoto => {} // Do nothing
                    };

                    sleep(Duration::from_millis(50)).await;
                }
            };

            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = thread_message_rx.recv() => { respond_to_msg!(msg); }

                _ = react_to_state_changes_in_loop => {}
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

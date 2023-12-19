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
    CompassDirection, CompassDirectionFlags, GenericServerToClientMsg, ServerToCameraMsg,
    ServerToControllerMsg,
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

    /// Cancel the current photo sequence.
    CancelPhotoSequence,

    /// We've received photo data from the camera.
    ReceivedPhoto {
        light_idx: u32,
        brightest_pixel_pos: (u32, u32),
        pixel_brightness: u8,
    },

    /// Finish scanning and produce a GIFT file.
    FinishScanning,
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
    let mut finished_sides = CompassDirectionFlags::empty();
    let mut pause_time = 0;
    let mut photo_map: HashMap<CompassDirection, Vec<((u32, u32), u8)>> = HashMap::new();

    info!("Beginning scan manager loop");

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
                                .expect_or_log("Should be able to send message down CAMERA_SEND");

                            CONTROLLER_SEND
                                .send(
                                    bincode::serialize(&ServerToControllerMsg::Generic(
                                        GenericServerToClientMsg::ServerReady,
                                    ))
                                    .expect_or_log("Should be able to serialize ServerReady"),
                                )
                                .expect_or_log(
                                    "Should be able to send message down CONTROLLER_SEND",
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

                                finished_sides.insert(current_camera_alignment.into());
                                debug!(
                                    ?current_camera_alignment,
                                    ?finished_sides,
                                    "Inserted alignment into finished_sides"
                                );
                                state = ScanManagerState::WaitingToScan;
                                driver
                                    .display_frame(FrameType::RawData(vec![[255; 3]; LIGHTS_NUM]));

                                CONTROLLER_SEND
                                    .send(
                                        bincode::serialize(
                                            &ServerToControllerMsg::PhotoSequenceDone {
                                                finished_sides,
                                            },
                                        )
                                        .expect_or_log(
                                            "Should be able to serialize PhotoSequenceDone",
                                        ),
                                    )
                                    .expect_or_log(
                                        "Should be able to send message down CONTROLLER_SEND",
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
                                .expect_or_log("Should be able to send message down CAMERA_SEND");

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
                msg = thread_message_rx.recv() => {
                    let msg = msg.expect_or_log("There should not be an error in receiving a ScanManagerMsg");
                    trace!(?msg, "Received ScanManagerMsg");

                    if respond_to_msg(
                        msg,
                        &mut connected_state,
                        &mut state,
                        &mut current_camera_alignment,
                        &mut finished_sides,
                        &mut pause_time,
                        &mut photo_map,
                    ).await {
                        crate::gift::generate_gift_file(photo_map);
                        crate::FINISHED_SCANNING.store(true, std::sync::atomic::Ordering::Relaxed);
                        return;
                    }
                }

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

async fn respond_to_msg(
    msg: ScanManagerMsg,
    connected_state: &mut ConnectedState,
    state: &mut ScanManagerState,
    current_camera_alignment: &mut CompassDirection,
    finished_sides: &mut CompassDirectionFlags,
    pause_time: &mut u16,
    photo_map: &mut HashMap<CompassDirection, Vec<((u32, u32), u8)>>,
) -> bool {
    match msg {
        ScanManagerMsg::CameraConnected => {
            connected_state.camera = true;
        }
        ScanManagerMsg::CameraDisconnected => {
            connected_state.camera = false;
            *state = ScanManagerState::WaitingForConnections;

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
            *state = ScanManagerState::WaitingForConnections;

            // We can fail to send if there's no camera connected
            let _ = CAMERA_SEND.send(
                bincode::serialize(&ServerToCameraMsg::Generic(
                    GenericServerToClientMsg::ServerNotReady,
                ))
                .expect_or_log("Should be able to serialize ServerNotReady"),
            );
        }
        ScanManagerMsg::StartTakingPhotos {
            camera_alignment,
            pause_time_ms,
        } => {
            CURRENT_IDX.store(0, Ordering::Relaxed);
            *current_camera_alignment = camera_alignment;
            *pause_time = pause_time_ms;

            photo_map
                .entry(*current_camera_alignment)
                .and_modify(|list| list.clear());

            CAMERA_SEND
                .send(
                    bincode::serialize(&ServerToCameraMsg::LockExposure)
                        .expect_or_log("Should be able to serialize LockExposure"),
                )
                .expect_or_log("Should be able to send message down CAMERA_SEND");

            // TODO: This is a bodge. We should ideally have a separate ScanManagerState for
            // waiting for the camera to acknowledge the exposure lock, but I'm too lazy to
            // implement that right now.
            sleep(Duration::from_millis(100)).await;

            *state = ScanManagerState::ReadyToTakePhoto;
        }
        ScanManagerMsg::CancelPhotoSequence => {
            info!(?current_camera_alignment, "Cancelling photo sequence");

            photo_map
                .entry(*current_camera_alignment)
                .and_modify(|list| list.clear());

            finished_sides.remove((*current_camera_alignment).into());
            debug!(
                ?current_camera_alignment,
                ?finished_sides,
                "Removed alignment from finished_sides"
            );
            *state = ScanManagerState::WaitingToScan;

            CONTROLLER_SEND
                .send(
                    bincode::serialize(&ServerToControllerMsg::PhotoSequenceCancelled {
                        finished_sides: *finished_sides,
                    })
                    .expect_or_log("Should be able to serialize PhotoSequenceDone"),
                )
                .expect_or_log("Should be able to send message down CONTROLLER_SEND");
        }
        ScanManagerMsg::ReceivedPhoto {
            light_idx,
            brightest_pixel_pos,
            pixel_brightness,
        } => {
            if *state == ScanManagerState::WaitingForPhoto {
                debug_assert_eq!(
                    light_idx,
                    CURRENT_IDX.load(Ordering::Relaxed) - 1,
                    "The camera and server should be in sync with the light indices"
                );

                info!(?light_idx, "Received photo from camera");

                photo_map
                    .entry(*current_camera_alignment)
                    .and_modify(|list| {
                        debug_assert_eq!(
                            list.len(),
                            light_idx as usize,
                            concat!(
                                "The light_idx should be in sync with the length ",
                                "of the corresponding vec in photo_map"
                            )
                        );
                        list.push((brightest_pixel_pos, pixel_brightness));
                    })
                    .or_insert_with(|| {
                        debug_assert_eq!(
                            light_idx, 0,
                            "If we're inserting a new vec into the map, the light_idx should be 0"
                        );
                        vec![(brightest_pixel_pos, pixel_brightness)]
                    });

                *state = ScanManagerState::ReadyToTakePhoto;

                debug!(?light_idx, "Updating controller with progress");
                CONTROLLER_SEND
                    .send(
                        bincode::serialize(&ServerToControllerMsg::ProgressUpdate {
                            scanned: light_idx as u16 + 1,
                            total: LIGHTS_NUM as u16,
                        })
                        .expect_or_log("Should be able to serialize ProgressUpdate"),
                    )
                    .expect_or_log("Should be able to send message down CONTROLLER_SEND");
            }
        }
        ScanManagerMsg::FinishScanning => {
            assert!(
                finished_sides.is_ready_to_finish(),
                "Controller should only send FinishScanning when we've scanned enough sides"
            );
            return true;
        }
    };

    false
}

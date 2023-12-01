//! This module provides [`CameraWidget`] for camera clients.

use crate::{app::AppState, generic_client::GenericClientWidget};
use egui::{Response, Ui};
use image::{ImageBuffer, Rgb};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType, Resolution},
    Camera,
};
use std::{
    fmt,
    time::{Duration, Instant},
};
use tracing::{debug, info, instrument};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{BasicCameraInfo, CameraToServerMsg, ServerToCameraMsg};

/// The API backend for this platform.
#[cfg(target_family = "wasm")]
const NOKHWA_API_BACKEND: ApiBackend = ApiBackend::Browser;

/// The API backend for this platform.
#[cfg(not(target_family = "wasm"))]
const NOKHWA_API_BACKEND: ApiBackend = ApiBackend::Auto;

/// Find the best camera on the device, if any. This function should only ever be called once.
fn find_best_camera() -> Option<Camera> {
    nokhwa::nokhwa_initialize(|_| {});

    let best_camera = nokhwa::query(NOKHWA_API_BACKEND)
        .expect_or_log("We should be able to query the available cameras on this device")
        .into_iter()
        .filter_map(|camera_info| {
            Camera::new(
                camera_info.index().clone(),
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution),
            )
            .or_else(|err| Err(err))
            .ok()
        })
        .max_by_key(|camera| {
            let Resolution { width_x, height_y } = camera.resolution();
            width_x * height_y
        });

    info!(idx = ?best_camera.as_ref().map(|cam| cam.index()), "Found best camera");
    best_camera
}

/// Get the [`BasicCameraInfo`] from a `nokhwa` [`Camera`].
fn get_basic_camera_info(camera: &Camera) -> BasicCameraInfo {
    let Resolution { width_x, height_y } = camera.resolution();
    BasicCameraInfo {
        resolution: (width_x, height_y),
    }
}

/// Get an image from the camera, panicking on failure.
fn get_image(camera: &mut Camera) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let frame = camera
        .frame()
        .expect_or_log("Should be able to get frame from camera");
    let image = frame
        .decode_image::<RgbFormat>()
        .expect_or_log("Should be able to decode image buffer");
    image
}

/// A widget to encapsulate a whole camera client.
#[derive(Debug)]
pub enum CameraWidget {
    /// This device has a working camera.
    Camera(InnerCameraWidget),

    /// This device doesn't have a working camera.
    NoCameraFound,
}

impl CameraWidget {
    /// Try to find a camera on this device and create the appropriate variant of [`CameraWidget`].
    pub fn new(async_runtime: prokio::Runtime) -> Self {
        match find_best_camera() {
            Some(camera) => Self::Camera(InnerCameraWidget::new(async_runtime, camera)),
            _ => Self::NoCameraFound,
        }
    }

    /// Forward the call to [`InnerCameraWidget::respond_to_server_messages`] or do nothing.
    #[inline]
    pub fn respond_to_server_messages(&mut self) -> Option<AppState> {
        match self {
            CameraWidget::Camera(inner) => inner.respond_to_server_messages(),
            CameraWidget::NoCameraFound => {
                // We have to tell the app that we're connected to the server if we don't have a camera
                // so that it defers rendering to [`CameraWidget`], which can display the error
                Some(AppState::Connected)
            }
        }
    }
}

impl egui::Widget for &mut CameraWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        match self {
            CameraWidget::Camera(inner) => inner.ui(ui),
            CameraWidget::NoCameraFound => ui.heading("ERROR: No camera found on this device"),
        }
    }
}

/// A widget to encapsulate a camera client that has a proper camera.
pub struct InnerCameraWidget {
    /// The inner widget that genericises background tasks.
    inner: GenericClientWidget<CameraToServerMsg, ServerToCameraMsg>,

    /// The camera belonging to this widget.
    camera: Camera,

    /// The amount of time between each frame.
    duration_between_frames: Duration,

    /// The time when the latest frame was taken.
    time_of_latest_frame: Instant,

    /// The image buffer for the latest frame.
    latest_frame: ImageBuffer<Rgb<u8>, Vec<u8>>,
}

impl fmt::Debug for InnerCameraWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        #[allow(
            dead_code,
            reason = "this struct is only used for formatting debug info"
        )]
        struct NokhwaCamera {
            idx: CameraIndex,
        }

        impl From<&Camera> for NokhwaCamera {
            fn from(value: &Camera) -> Self {
                NokhwaCamera {
                    idx: value.index().clone(),
                }
            }
        }

        f.debug_struct("InnerCameraWidget")
            .field("inner", &self.inner)
            .field("camera", &NokhwaCamera::from(&self.camera))
            .field("duration_between_frames", &self.duration_between_frames)
            .field("time_of_latest_frame", &self.time_of_latest_frame)
            .field("latest_frame", &..)
            .finish()
    }
}

impl InnerCameraWidget {
    /// Create a new [`InnerCameraWidget`] and initialise background tasks.
    fn new(async_runtime: prokio::Runtime, mut camera: Camera) -> Self {
        let inner = GenericClientWidget::new(async_runtime, || {
            CameraToServerMsg::EstablishConnection(get_basic_camera_info(&camera))
        });

        camera
            .open_stream()
            .expect_or_log("Should be able to open stream on camera");

        let fps = camera.frame_rate();
        let duration_between_frames = Duration::from_micros((1_000_000.0 / fps as f64) as u64);

        let time_of_latest_frame = Instant::now();

        let latest_frame = get_image(&mut camera);

        Self {
            inner,
            camera,
            duration_between_frames,
            time_of_latest_frame,
            latest_frame,
        }
    }

    /// Respond to all the messages from the server that are in the queue and return the new
    /// [`AppState`] for the top level app, if any.
    #[instrument(skip_all)]
    fn respond_to_server_messages(&mut self) -> Option<AppState> {
        let mut new_state = None;

        while let Ok(msg) = self.inner.server_rx.try_recv() {
            debug!(?msg, "Responding to server message");

            match msg {
                ServerToCameraMsg::Generic(msg) => {
                    new_state = Some(self.inner.respond_to_generic_server_message(msg));
                }
                ServerToCameraMsg::TakePhoto { id: _ } => todo!("Respond to TakePhoto"),
            }
        }

        new_state
    }

    /// Refresh [`self.latest_frame`] if it needs to be refreshed.
    fn refresh_frame(&mut self) {
        if self.time_of_latest_frame.elapsed() >= self.duration_between_frames {
            debug!("Refreshing latest_frame");
            self.latest_frame = get_image(&mut self.camera);
            self.time_of_latest_frame = Instant::now();
        }
    }

    /// Display the UI for when the camera is connected and the server is ready to scan.
    fn display_main_ui(&mut self, ui: &mut Ui) -> Response {
        self.refresh_frame();
        ui.label("Server ready")
    }
}

impl egui::Widget for &mut InnerCameraWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        use crate::generic_client::GenericClientState as State;

        self.respond_to_server_messages();

        let state = *self
            .inner
            .state
            .read()
            .expect_or_log("Should be able to read from client widget state");
        match state {
            State::WaitingForConnection => ui.centered_and_justified(|ui| ui.spinner()).response,
            State::Rejected => {
                ui.centered_and_justified(|ui| ui.label("Another camera is already connected"))
                    .response
            }
            State::ServerNotReady => {
                ui.horizontal_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("Waiting for a controller client");
                        ui.spinner();
                    })
                    .response
                })
                .response
            }
            State::ServerReady => self.display_main_ui(ui),
        }
    }
}

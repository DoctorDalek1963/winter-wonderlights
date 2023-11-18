//! This module provides [`CameraWidget`] for camera clients.

use crate::{app::AppState, generic_client::GenericClientWidget};
use egui::{Response, Ui};
use nokhwa::{
    pixel_format::LumaFormat,
    utils::{ApiBackend, RequestedFormat, RequestedFormatType, Resolution},
    Camera,
};
use std::cell::OnceCell;
use tracing::{debug, info, instrument};
use tracing_unwrap::{OptionExt, ResultExt};
use ww_scanner_shared::{BasicCameraInfo, CameraToServerMsg, ServerToCameraMsg};

/// The best camera available on this device, if any.
const BEST_CAMERA: OnceCell<Option<Camera>> = OnceCell::new();

/// Find the best camera on the device, if any, and set [`BEST_CAMERA`] accordingly.
fn find_best_camera() {
    let _ = BEST_CAMERA.set(
        nokhwa::query(ApiBackend::Auto)
            .expect_or_log("We should be able to query the available cameras on this device")
            .into_iter()
            .filter_map(|camera_info| {
                Camera::new(
                    camera_info.index().clone(),
                    RequestedFormat::new::<LumaFormat>(
                        RequestedFormatType::AbsoluteHighestResolution,
                    ),
                )
                .ok()
            })
            .max_by_key(|camera| {
                let Resolution { width_x, height_y } = camera.resolution();
                width_x * height_y
            }),
    );

    info!(best_camera = ?BEST_CAMERA.get().map(|opt_cam| opt_cam.as_ref().map(|cam| cam.info().human_name())), "Found best camera");
}

/// Get the [`BasicCameraInfo`] from a `nokhwa` [`Camera`].
fn get_basic_camera_info(camera: &Camera) -> BasicCameraInfo {
    let Resolution { width_x, height_y } = camera.resolution();
    BasicCameraInfo {
        resolution: (width_x, height_y),
    }
}

/// Get a `&'static Camera` to [`BEST_CAMERA`], panicking if this device doesn't have a camera.
macro_rules! get_best_camera {
    () => {
        BEST_CAMERA
            .get()
            .expect_or_log("find_best_camera() should have been called first")
            .as_ref()
            .expect_or_log("get_best_camera() should only be called when the device has a camera")
    };
}

/// A widget to encapsulate a whole camera client.
#[derive(Clone, Debug)]
pub enum CameraWidget {
    /// This device has a working camera.
    Camera(InnerCameraWidget),

    /// This device doesn't have a working camera.
    NoCameraFound,
}

impl CameraWidget {
    /// Try to find a camera on this device and create the appropriate variant of [`CameraWidget`].
    pub fn new(async_runtime: prokio::Runtime) -> Self {
        find_best_camera();
        match BEST_CAMERA.get() {
            Some(Some(_camera)) => Self::Camera(InnerCameraWidget::new(async_runtime)),
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
#[derive(Clone, Debug)]
pub struct InnerCameraWidget {
    /// The inner widget that genericises background tasks.
    inner: GenericClientWidget<CameraToServerMsg, ServerToCameraMsg>,
}

impl InnerCameraWidget {
    /// Create a new [`InnerCameraWidget`] and initialise background tasks.
    fn new(async_runtime: prokio::Runtime) -> Self {
        let inner = GenericClientWidget::new(async_runtime, || {
            CameraToServerMsg::EstablishConnection(get_basic_camera_info(get_best_camera!()))
        });
        Self { inner }
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

    /// Display the UI for when the camera is connected and the server is ready to scan.
    fn display_main_ui(&mut self, ui: &mut Ui) -> Response {
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

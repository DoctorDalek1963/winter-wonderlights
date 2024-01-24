//! This module provides [`CameraWidget`] for camera clients.

use crate::{app::AppState, generic_client::GenericClientWidget};
use egui::{
    load::{SizedTexture, TextureLoader},
    ColorImage, Context, ImageData, Response, TextureOptions, Ui,
};
use image::{imageops, GrayImage, Luma};
use nokhwa::{
    pixel_format::LumaFormat,
    utils::{
        ApiBackend, CameraIndex, KnownCameraControl, RequestedFormat, RequestedFormatType,
        Resolution,
    },
    Camera,
};
use std::{
    fmt,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, info, instrument, trace, warn};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{BasicCameraInfo, CameraToServerMsg, ServerToCameraMsg};

/// The API backend for this platform.
#[cfg(target_family = "wasm")]
const NOKHWA_API_BACKEND: ApiBackend = ApiBackend::Browser;

/// The API backend for this platform.
#[cfg(not(target_family = "wasm"))]
const NOKHWA_API_BACKEND: ApiBackend = ApiBackend::Auto;

/// A rotation of a number of degrees clockwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rotation {
    /// The image is exactly as it came from the camera.
    Original,

    /// The image is rotated 90 degrees clockwise.
    Rotate90,

    /// The image is rotated 180 degrees.
    Rotate180,

    /// The image is rotated 270 degrees clockwise (90 degrees anti-clockwise).
    Rotate270,
}

impl Rotation {
    /// Rotate by 90 degrees clockwise.
    fn next_clockwise(self) -> Self {
        match self {
            Self::Original => Self::Rotate90,
            Self::Rotate90 => Self::Rotate180,
            Self::Rotate180 => Self::Rotate270,
            Self::Rotate270 => Self::Original,
        }
    }

    /// Rotate by 90 degrees anti-clockwise.
    fn next_anticlockwise(self) -> Self {
        match self {
            Self::Original => Self::Rotate270,
            Self::Rotate90 => Self::Original,
            Self::Rotate180 => Self::Rotate90,
            Self::Rotate270 => Self::Rotate180,
        }
    }
}

/// Find the best camera on the device, if any. This function should only ever be called once.
fn find_best_camera() -> Option<Camera> {
    nokhwa::nokhwa_initialize(|_| {});

    let best_camera = nokhwa::query(NOKHWA_API_BACKEND)
        .expect_or_log("We should be able to query the available cameras on this device")
        .into_iter()
        .filter_map(|camera_info| {
            Camera::new(
                camera_info.index().clone(),
                RequestedFormat::new::<LumaFormat>(RequestedFormatType::AbsoluteHighestResolution),
            )
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
#[inline]
fn get_basic_camera_info(camera: &Camera) -> BasicCameraInfo {
    let Resolution { width_x, height_y } = camera.resolution();
    BasicCameraInfo {
        resolution: (width_x, height_y),
    }
}

/// Get an image from the camera, panicking on failure.
fn get_image(camera: &mut Camera, rotation: Rotation) -> GrayImage {
    let frame = camera
        .frame()
        .expect_or_log("Should be able to get frame from camera");
    let image = frame
        .decode_image::<LumaFormat>()
        .expect_or_log("Should be able to decode image buffer");

    match rotation {
        Rotation::Original => image,
        Rotation::Rotate90 => imageops::rotate90(&image),
        Rotation::Rotate180 => imageops::rotate180(&image),
        Rotation::Rotate270 => imageops::rotate270(&image),
    }
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
    pub fn new(async_runtime: prokio::Runtime, ctx: &Context) -> Self {
        match find_best_camera() {
            Some(camera) => Self::Camera(InnerCameraWidget::new(async_runtime, camera, ctx)),
            _ => Self::NoCameraFound,
        }
    }

    /// Forward the call to [`InnerCameraWidget::respond_to_server_messages`] or do nothing.
    #[inline]
    pub fn respond_to_server_messages(&mut self) -> Option<AppState> {
        match self {
            Self::Camera(inner) => inner.respond_to_server_messages(),
            Self::NoCameraFound => {
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
    latest_frame: Arc<RwLock<GrayImage>>,

    /// The rotation of the camera from the source.
    rotation: Rotation,

    /// Has the exposure been locked?
    exposure_locked: bool,
}

impl fmt::Debug for InnerCameraWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(
            dead_code,
            clippy::missing_docs_in_private_items,
            reason = "this struct is only used for formatting debug info"
        )]
        struct DebuggableNokhwaCamera {
            idx: CameraIndex,
        }

        impl From<&Camera> for DebuggableNokhwaCamera {
            fn from(value: &Camera) -> Self {
                Self {
                    idx: value.index().clone(),
                }
            }
        }

        impl fmt::Debug for DebuggableNokhwaCamera {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("Camera").field("idx", &self.idx).finish()
            }
        }

        f.debug_struct("InnerCameraWidget")
            .field("inner", &self.inner)
            .field("camera", &DebuggableNokhwaCamera::from(&self.camera))
            .field("duration_between_frames", &self.duration_between_frames)
            .field("time_of_latest_frame", &self.time_of_latest_frame)
            .field("latest_frame", &..)
            .field("latest_frame", &self.rotation)
            .field("latest_frame", &self.exposure_locked)
            .finish()
    }
}

/// A simple struct to let `egui` load textures from the webcam provided by `nokhwa`.
struct NokhwaTextureLoader(Arc<RwLock<GrayImage>>);

impl TextureLoader for NokhwaTextureLoader {
    fn id(&self) -> &str {
        concat!(std::module_path!(), "::NokhwaTextureLoader")
    }

    fn load(
        &self,
        ctx: &egui::Context,
        uri: &str,
        _texture_options: egui::TextureOptions,
        _size_hint: egui::SizeHint,
    ) -> egui::load::TextureLoadResult {
        if uri.starts_with("nokhwacamera://") {
            let buf = self.0.read().unwrap_or_log();
            let (w, h) = buf.dimensions();
            let image =
                ColorImage::from_gray([w as usize, h as usize], buf.as_flat_samples().as_slice());
            let texture = SizedTexture::from_handle(&ctx.load_texture(
                "nokhwa_image",
                ImageData::Color(Arc::new(image)),
                TextureOptions::default(),
            ));
            Ok(egui::load::TexturePoll::Ready { texture })
        } else {
            Err(egui::load::LoadError::NotSupported)
        }
    }

    fn forget(&self, _uri: &str) {}

    fn forget_all(&self) {}

    fn byte_size(&self) -> usize {
        0
    }
}

impl InnerCameraWidget {
    /// Create a new [`InnerCameraWidget`] and initialise background tasks.
    fn new(async_runtime: prokio::Runtime, mut camera: Camera, ctx: &Context) -> Self {
        let inner = GenericClientWidget::new(async_runtime, || {
            CameraToServerMsg::EstablishConnection(get_basic_camera_info(&camera))
        });

        camera
            .open_stream()
            .expect_or_log("Should be able to open stream on camera");

        let fps = camera.frame_rate();
        let duration_between_frames = Duration::from_micros((1_000_000.0 / fps as f64) as u64);

        let time_of_latest_frame = Instant::now();

        let latest_frame = Arc::new(RwLock::new(get_image(&mut camera, Rotation::Original)));

        let loader = NokhwaTextureLoader(Arc::clone(&latest_frame));
        ctx.add_texture_loader(Arc::new(loader));

        Self {
            inner,
            camera,
            duration_between_frames,
            time_of_latest_frame,
            latest_frame,
            rotation: Rotation::Original,
            exposure_locked: false,
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
                ServerToCameraMsg::LockExposure => self.lock_exposure(),
                ServerToCameraMsg::TakePhoto { light_idx } => {
                    info!(?light_idx, "Taking photo");

                    let (brightest_pixel_pos, pixel_brightness) = self.get_brightest_pixel();
                    self.inner.send_msg(CameraToServerMsg::PhotoTaken {
                        light_idx,
                        brightest_pixel_pos,
                        pixel_brightness,
                    });
                }
            }
        }

        new_state
    }

    /// Lock the exposure of the camera to avoid auto-exposure messing with relative brightnesses.
    #[instrument(skip_all)]
    fn lock_exposure(&mut self) {
        if self.exposure_locked {
            trace!("Exposure already locked, so not re-locking it");
            return;
        }

        if let Ok(_control) = self.camera.camera_control(KnownCameraControl::Exposure) {
            warn!("TODO: Lock exposure");
        } else {
            warn!("Unable to lock exposure on this camera");
        };

        self.exposure_locked = true;
    }

    /// Get the position of the brightest pixel in the current image and it's brightness.
    ///
    /// Returns a tuple `(x, y)`, where (0, 0) is the top left of the image, along with
    fn get_brightest_pixel(&self) -> ((u32, u32), u8) {
        let (x, y, pixel_brightness) = self
            .latest_frame
            .read()
            .unwrap_or_log()
            .enumerate_pixels()
            .fold(
                (0, 0, 0),
                |(acc_x, acc_y, acc_pixel), (x, y, &Luma([pixel]))| {
                    if pixel > acc_pixel {
                        (x, y, pixel)
                    } else {
                        (acc_x, acc_y, acc_pixel)
                    }
                },
            );
        ((x, y), pixel_brightness)
    }

    /// Refresh [`self.latest_frame`] if it needs to be refreshed.
    fn refresh_frame(&mut self) {
        if self.time_of_latest_frame.elapsed() >= self.duration_between_frames {
            trace!("Refreshing latest_frame");
            *self
                .latest_frame
                .write()
                .expect_or_log("latest_frame should not have been poisoned") =
                get_image(&mut self.camera, self.rotation);
            self.time_of_latest_frame = Instant::now();
        }
    }

    /// Display the UI for when the camera is connected and the server is ready to scan.
    fn display_main_ui(&mut self, ui: &mut Ui) -> Response {
        self.refresh_frame();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Rotate image:");
                if ui.button("⟲").clicked() {
                    self.rotation = self.rotation.next_anticlockwise();
                }
                if ui.button("⟳").clicked() {
                    self.rotation = self.rotation.next_clockwise();
                }
            });

            ui.image("nokhwacamera://");
        })
        .response
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

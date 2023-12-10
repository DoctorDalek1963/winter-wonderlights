//! This module provides [`ControllerWidget`] for controller clients.

use crate::{app::AppState, generic_client::GenericClientWidget};
use egui::{Response, Ui};
use strum::IntoEnumIterator;
use tracing::{debug, instrument};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{CompassDirection, ControllerToServerMsg, ServerToControllerMsg};

/// A widget to encapsulate a whole controller client.
#[derive(Clone, Debug)]
pub struct ControllerWidget {
    /// The inner widget that genericises background tasks.
    inner: GenericClientWidget<ControllerToServerMsg, ServerToControllerMsg>,

    /// The direction of the tree which is currently facing the camera.
    direction: CompassDirection,

    /// Are we ready to take photos? Or are we waiting for the camera to finish?
    ready_to_take_photos: bool,

    /// The time to pause between taking photos, in milliseconds.
    pause_time_ms: u16,

    /// The progress that the server has made scanning the lights. See
    /// [`ServerToControllerMsg::ProgressUpdate`].
    progress: (u16, u16),
}

impl ControllerWidget {
    /// Create a new [`ControllerWidget`] and initialise background tasks.
    pub fn new(async_runtime: prokio::Runtime) -> Self {
        let inner =
            GenericClientWidget::new(async_runtime, || ControllerToServerMsg::EstablishConnection);
        Self {
            inner,
            direction: CompassDirection::South,
            ready_to_take_photos: true,
            pause_time_ms: 50,
            progress: (0, 0),
        }
    }

    /// Respond to all the messages from the server that are in the queue and return the new
    /// [`AppState`] for the top level app, if any.
    #[instrument(skip_all)]
    pub fn respond_to_server_messages(&mut self) -> Option<AppState> {
        let mut new_state = None;

        while let Ok(msg) = self.inner.server_rx.try_recv() {
            debug!(?msg, "Responding to server message");

            match msg {
                ServerToControllerMsg::Generic(msg) => {
                    new_state = Some(self.inner.respond_to_generic_server_message(msg));
                }
                ServerToControllerMsg::PhotoSequenceDone => {
                    self.ready_to_take_photos = true;
                    self.progress.0 = 0;
                }
                ServerToControllerMsg::ProgressUpdate { scanned, total } => {
                    self.progress = (scanned, total);
                }
            }
        }

        new_state
    }

    /// Display the UI for when the controller is connected and the server is ready to scan.
    fn display_main_ui(&mut self, ui: &mut Ui) -> Response {
        const UI_SPACING: f32 = 20.0;

        ui.vertical(|ui| {
            if ui
                .add_enabled(self.ready_to_take_photos, |ui: &mut Ui| {
                    ui.button("Start taking photos")
                })
                .clicked()
            {
                self.inner
                    .send_msg(ControllerToServerMsg::ReadyToTakePhotos {
                        camera_alignment: self.direction,
                        pause_time_ms: self.pause_time_ms,
                    });
                self.ready_to_take_photos = false;
            }

            if !self.ready_to_take_photos {
                ui.add_space(UI_SPACING);

                let (scanned, total) = self.progress;
                ui.add(
                    egui::ProgressBar::new(scanned as f32 / total as f32)
                        .text(format!("{scanned}/{total} lights scanned")),
                );

                if ui.button("Cancel photo sequence").clicked() {
                    self.inner
                        .send_msg(ControllerToServerMsg::CancelPhotoSequence);
                    self.ready_to_take_photos = true;
                }
            }

            ui.add_space(UI_SPACING);
            ui.add(
                egui::Slider::new(&mut self.pause_time_ms, 0..=1000)
                    .clamp_to_range(false)
                    .suffix("ms")
                    .text("Pause time between photos"),
            );
            ui.label("(NOTICE: increase this if you don't like flashing lights)");
            ui.add_space(UI_SPACING);

            egui::ComboBox::from_label("Side of tree facing camera")
                .selected_text(self.direction.name())
                .show_ui(ui, |ui| {
                    for direction in CompassDirection::iter() {
                        ui.selectable_value(&mut self.direction, direction, direction.name());
                    }
                });
        })
        .response
    }
}

impl egui::Widget for &mut ControllerWidget {
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
                ui.centered_and_justified(|ui| ui.label("Another controller is already connected"))
                    .response
            }
            State::ServerNotReady => {
                ui.horizontal_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("Waiting for a camera client");
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

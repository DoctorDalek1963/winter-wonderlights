//! This module provides [`ControllerWidget`] for controller clients.

use crate::{app::AppState, generic_client::GenericClientWidget};
use egui::{Response, Ui};
use tracing::{debug, instrument};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{ControllerToServerMsg, ServerToControllerMsg};

/// A widget to encapsulate a whole controller client.
#[derive(Clone, Debug)]
pub struct ControllerWidget {
    /// The inner widget that genericises background tasks.
    inner: GenericClientWidget<ControllerToServerMsg, ServerToControllerMsg>,
}

impl ControllerWidget {
    /// Create a new [`CameraWidget`] and initialise background tasks.
    pub fn new(async_runtime: prokio::Runtime) -> Self {
        let inner = GenericClientWidget::new(async_runtime);
        Self { inner }
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
                ServerToControllerMsg::PhotoSequenceDone => todo!("Respond to PhotoSequenceDone"),
            }
        }

        new_state
    }

    /// Display the UI for when the controller is connected and the server is ready to scan.
    fn display_main_ui(&mut self, ui: &mut Ui) -> Response {
        ui.label("Server ready")
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

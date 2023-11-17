//! This module handles the [`App`] type for the `eframe`-based GUI.

use crate::{camera::CameraWidget, controller::ControllerWidget};
use eframe::egui::Context;
use std::sync::{Arc, RwLock};
use tracing::{instrument, warn};
use tracing_unwrap::ResultExt;

/// The current state of the app and its connection to the server.
#[derive(Clone, Debug)]
pub enum AppState {
    /// We're waiting for the user to choose which type of client this will be.
    WaitingForChoice,

    /// We're currently waiting to connect to the server.
    WaitingForConnection,

    /// We're connected to the server.
    Connected,

    /// We were rejected by the server because another client is currently connected.
    Rejected,
}

impl Default for AppState {
    fn default() -> Self {
        Self::WaitingForChoice
    }
}

/// The state of the client widgets.
#[derive(Clone, Debug)]
struct ClientWidgetsState {
    /// A widget for the camera client.
    camera_widget: Option<CameraWidget>,

    /// A widget for the controller client.
    controller_widget: Option<ControllerWidget>,
}

impl Default for ClientWidgetsState {
    fn default() -> Self {
        Self {
            camera_widget: None,
            controller_widget: None,
        }
    }
}

/// The app type itself.
#[derive(Clone, Debug)]
pub struct App {
    /// The state of the app.
    app_state: Arc<RwLock<AppState>>,

    /// The state of the client widgets.
    client_state: Arc<RwLock<ClientWidgetsState>>,

    /// An async runtime used to send async messages.
    async_runtime: prokio::Runtime,
}

impl App {
    /// Create a new [`App`].
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let app_state = Arc::new(RwLock::new(AppState::default()));
        let client_state = Arc::new(RwLock::new(ClientWidgetsState::default()));
        let async_runtime = prokio::Runtime::default();
        Self {
            app_state,
            client_state,
            async_runtime,
        }
    }

    /// Display the GUI for waiting for the user to choose a type of client.
    #[instrument(skip_all)]
    fn display_gui_waiting_for_choice(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.vertical(|ui| {
                ui.label("Please choose what type of client this device should be:");

                if ui.button("Camera").clicked() {
                    self.client_state.write().unwrap_or_log().camera_widget = Some(CameraWidget::new(self.async_runtime.clone()));
                    *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                }

                if ui.button("Controller").clicked() {
                    self.client_state.write().unwrap_or_log().controller_widget = Some(ControllerWidget::new(self.async_runtime.clone()));
                    *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                }

                if ui.button("Both").clicked() {
                    warn!("Clients with both camera and controller are not currently implemented properly");
                    self.client_state.write().unwrap_or_log().camera_widget = Some(CameraWidget::new(self.async_runtime.clone()));
                    self.client_state.write().unwrap_or_log().controller_widget = Some(ControllerWidget::new(self.async_runtime.clone()));
                    *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                }
            }));
        });
    }

    /// Display the GUI for waiting for a connection to the server.
    fn display_gui_waiting_for_connection(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.spinner());
        });
    }

    /// Display the GUI for waiting for a connection to the server.
    fn display_gui_rejected(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Connection to server rejected. Please try again.")
            });
        });
    }

    /// Display the GUI for when we're connected to the server.
    #[instrument(skip_all)]
    fn display_gui_connected(&mut self, ctx: &Context) {
        let mut client_state = self.client_state.write().unwrap_or_log();

        if let Some(ref mut _camera) = client_state.camera_widget
            && let Some(ref mut _controller) = client_state.controller_widget
        {
            unimplemented!("Using both client types not yet implemented");
        } else if let Some(ref mut camera) = client_state.camera_widget {
            camera.respond_to_server_messages();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| ui.add(camera));
            });
        } else if let Some(ref mut controller) = client_state.controller_widget {
            controller.respond_to_server_messages();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| ui.add(controller));
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("ERROR: Connected to the server but no client initialised")
                });
            });
        }
    }

    /// Respond to the server messages in the client widgets.
    fn respond_to_server_messages_in_client_widgets(&mut self) {
        let mut client_state = self.client_state.write().unwrap_or_log();

        if let Some(ref mut camera) = client_state.camera_widget {
            if let Some(new_state) = camera.respond_to_server_messages() {
                *self.app_state.write().unwrap_or_log() = new_state;
            }
        }

        if let Some(ref mut controller) = client_state.controller_widget {
            if let Some(new_state) = controller.respond_to_server_messages() {
                *self.app_state.write().unwrap_or_log() = new_state;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.respond_to_server_messages_in_client_widgets();

        let app_state = self.app_state.read().unwrap_or_log().clone();
        match app_state {
            AppState::WaitingForChoice => self.display_gui_waiting_for_choice(ctx),
            AppState::WaitingForConnection => self.display_gui_waiting_for_connection(ctx),
            AppState::Connected { .. } => self.display_gui_connected(ctx),
            AppState::Rejected => self.display_gui_rejected(ctx),
        };

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed
        ctx.request_repaint();
    }
}

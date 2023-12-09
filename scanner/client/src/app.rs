//! This module handles the [`App`] type for the `eframe`-based GUI.

use crate::{camera::CameraWidget, controller::ControllerWidget};
use eframe::egui::Context;
use egui::Vec2;
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

/// The app type itself.
#[derive(Debug)]
pub struct App {
    /// The state of the app.
    app_state: Arc<RwLock<AppState>>,

    /// Do we have a camera widget?
    camera_widget: Option<CameraWidget>,

    /// Do we have a controller widget?
    controller_widget: Option<ControllerWidget>,

    /// An async runtime used to send async messages.
    async_runtime: prokio::Runtime,
}

impl App {
    /// Create a new [`App`].
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let app_state = Arc::new(RwLock::new(AppState::default()));
        let async_runtime = prokio::Runtime::default();
        Self {
            app_state,
            camera_widget: None,
            controller_widget: None,
            async_runtime,
        }
    }

    /// Display the GUI for waiting for the user to choose a type of client.
    #[instrument(skip_all)]
    fn display_gui_waiting_for_choice(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.vertical(|ui| {
                    ui.label("Please choose what type of client this device should be:");

                    if ui.button("Camera").clicked() {
                        self.camera_widget =
                            Some(CameraWidget::new(self.async_runtime.clone(), ctx));
                        *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                    }

                    if ui.button("Controller").clicked() {
                        self.controller_widget =
                            Some(ControllerWidget::new(self.async_runtime.clone()));
                        *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                    }

                    if ui.button("Both").clicked() {
                        self.camera_widget =
                            Some(CameraWidget::new(self.async_runtime.clone(), ctx));
                        self.controller_widget =
                            Some(ControllerWidget::new(self.async_runtime.clone()));
                        *self.app_state.write().unwrap_or_log() = AppState::WaitingForConnection;
                    }
                })
            });
        });
    }

    /// Display the GUI for waiting for a connection to the server.
    fn display_gui_waiting_for_connection(ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.spinner());
        });
    }

    /// Display the GUI for waiting for a connection to the server.
    fn display_gui_rejected(ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Connection to server rejected. Please try again.")
            });
        });
    }

    /// Display the GUI for when we're connected to the server.
    #[instrument(skip_all)]
    fn display_gui_connected(&mut self, ctx: &Context) {
        if let Some(ref mut camera) = self.camera_widget
            && let Some(ref mut controller) = self.controller_widget
        {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical(|ui| {
                    // The camera is only allowed to fill 75% of the vertical space. Without this
                    // resriction, it would try to fill as much space as possible and push the
                    // controller off the bottom of the window
                    ui.allocate_ui(ui.max_rect().size() * Vec2::new(1.0, 0.75), |ui| {
                        ui.add(camera)
                    });

                    ui.separator();
                    ui.add(controller);
                });
            });
        } else if let Some(ref mut camera) = self.camera_widget {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| ui.add(camera));
            });
        } else if let Some(ref mut controller) = self.controller_widget {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| ui.add(controller));
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.heading("ERROR: Connected to the server but no client initialised")
                });
            });
        }
    }

    /// Respond to the server messages in the client widgets.
    fn respond_to_server_messages_in_client_widgets(&mut self) {
        if let Some(ref mut camera) = self.camera_widget {
            if let Some(new_state) = camera.respond_to_server_messages() {
                *self.app_state.write().unwrap_or_log() = new_state;
            }
        }

        if let Some(ref mut controller) = self.controller_widget {
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
            AppState::WaitingForConnection => Self::display_gui_waiting_for_connection(ctx),
            AppState::Connected { .. } => self.display_gui_connected(ctx),
            AppState::Rejected => Self::display_gui_rejected(ctx),
        };

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed
        ctx.request_repaint();
    }
}

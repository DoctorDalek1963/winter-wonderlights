//! This module handles the [`App`] type for the `eframe`-based GUI.

use eframe::egui::{self, Context};

/// The app type itself.
pub struct App;

impl App {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self {}
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello!!!");
        });
    }
}

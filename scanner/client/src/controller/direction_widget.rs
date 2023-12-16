//! This module just provides a widget to allow the user to select a direction. See
//! [`direction_widget`].

use egui::{Response, Ui, Widget};
use strum::IntoEnumIterator;
use ww_scanner_shared::{CompassDirection, CompassDirectionFlags};

/// The internal logic of [`direction_widget`].
fn direction_widget_ui(
    ui: &mut Ui,
    selected_direction: &mut CompassDirection,
    scanned_directions: &mut CompassDirectionFlags,
) -> Response {
    egui::ComboBox::from_label("Side of tree facing camera")
        .selected_text(selected_direction.name())
        .show_ui(ui, |ui| {
            for direction in CompassDirection::iter() {
                ui.selectable_value(selected_direction, direction, direction.name());
            }
        })
        .response
}

/// A widget that allows the user to select a direction and see which directions they've already
/// selected.
pub fn direction_widget<'a>(
    selected_direction: &'a mut CompassDirection,
    scanned_directions: &'a mut CompassDirectionFlags,
) -> impl Widget + 'a {
    |ui: &mut Ui| direction_widget_ui(ui, selected_direction, scanned_directions)
}

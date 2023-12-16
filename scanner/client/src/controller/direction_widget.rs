//! This module just provides a widget to allow the user to select a direction. See
//! [`direction_widget`].

use egui::{Align2, Response, Sense, TextStyle, Ui, Vec2, Widget};
use ww_scanner_shared::{CompassDirection, CompassDirectionFlags};

/// π/4.
const QP: f32 = std::f32::consts::PI / 4.;

/// A widget that allows the user to select a direction and see which directions they've already
/// selected.
pub fn direction_widget<'a>(
    selected_direction: &'a mut CompassDirection,
    scanned_directions: &'a mut CompassDirectionFlags,
) -> impl Widget + 'a {
    |ui: &mut Ui| direction_widget_ui(ui, selected_direction, scanned_directions)
}

/// The internal logic of [`direction_widget`].
fn direction_widget_ui(
    ui: &mut Ui,
    selected_direction: &mut CompassDirection,
    scanned_directions: &mut CompassDirectionFlags,
) -> Response {
    let button_diameter = ui.spacing().interact_size.y * 1.5;
    let half_line_length = 2. * button_diameter;
    let compass_diameter = 10. * button_diameter;

    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::splat(compass_diameter), Sense::click_and_drag());
    let center = rect.center();

    // TODO: Respond to interaction

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let cross_stroke = visuals.fg_stroke;

        let cross_north_vec = rotate(
            Vec2::new(0., -half_line_length),
            (4i8 - selected_direction.turns_from_north() as i8) as f32 * QP,
        );

        ui.painter().arrow(center, cross_north_vec, cross_stroke);
        ui.painter()
            .line_segment([center, center - cross_north_vec], cross_stroke);
        ui.painter().line_segment(
            [
                center + cross_north_vec.rot90(),
                center - cross_north_vec.rot90(),
            ],
            cross_stroke,
        );

        let font_id = TextStyle::Heading.resolve(ui.style());
        ui.painter().text(
            center + cross_north_vec * 1.3,
            Align2::CENTER_CENTER,
            "N",
            font_id.clone(),
            visuals.text_color(),
        );
        ui.painter().text(
            center + 1.3 * cross_north_vec.rot90().rot90().rot90(),
            Align2::CENTER_CENTER,
            "E",
            font_id.clone(),
            visuals.text_color(),
        );
        ui.painter().text(
            center + 1.3 * cross_north_vec.rot90().rot90(),
            Align2::CENTER_CENTER,
            "S",
            font_id.clone(),
            visuals.text_color(),
        );
        ui.painter().text(
            center + 1.3 * cross_north_vec.rot90(),
            Align2::CENTER_CENTER,
            "W",
            font_id.clone(),
            visuals.text_color(),
        );
    }

    response
}

/// Rotate a [`Vec2`] by 45 degrees clockwise.
fn rot45(v: Vec2) -> Vec2 {
    rotate(v, QP)
}

/// Rotate the given vector clockwise by the given angle (in radians).
fn rotate(v: Vec2, angle: f32) -> Vec2 {
    Vec2::angled(v.angle() + angle) * v.length()
}

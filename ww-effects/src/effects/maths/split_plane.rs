//! This module contains the [`SplitPlane`] effect.

#[cfg(feature = "config-impls")]
pub use config::SplitPlaneConfig;

#[cfg(feature = "effect-impls")]
pub use effect::SplitPlane;

use crate::effects::prelude::*;

#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`SplitPlane`] effect.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Sealed)]
    pub struct SplitPlaneConfig {
        /// The colour of one side of the split plane.
        pub side_a_colour: RGBArray,

        /// The colour of the other side of the split plane.
        pub side_b_colour: RGBArray,

        /// The speed of rotation around the axis of rotation, measured in radians per second in
        /// the anti-clockwise direction.
        pub rotation_speed: f32,

        /// The axis of rotation starts as the x-axis, centered vertically in the middle of the
        /// tree. This variable rotates that axis of rotation about the z-axis. Measured in
        /// degrees.
        pub rotation_axis_z_rotation_degrees: f32,

        /// The rotation axis is always parallel to the floor. When this value is 0, the rotation
        /// axis will be vertically in the middle of the tree. Positive values move the rotation
        /// axis up and negative values move it down. Measured in GIFT coordinate space, so a
        /// distance of 1 is the radius of the base of the tree.
        pub rotation_axis_z_height_offset: f32,
    }

    impl Default for SplitPlaneConfig {
        fn default() -> Self {
            Self {
                side_a_colour: [244, 29, 9],
                side_b_colour: [26, 234, 23],
                rotation_speed: 1.,
                rotation_axis_z_rotation_degrees: 0.,
                rotation_axis_z_height_offset: 0.,
            }
        }
    }

    impl EffectConfig for SplitPlaneConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            ui.label(RichText::new("SplitPlane config").heading());

            let mut config_changed = false;

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.rotation_speed, -3.0..=3.0)
                        .suffix("rad/s")
                        .clamp_to_range(false)
                        .text("Rotation speed"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.rotation_axis_z_rotation_degrees, 0.0..=360.0)
                        .suffix("°")
                        .text("Rotation axis z rotation"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.rotation_axis_z_height_offset, -2.0..=2.0)
                        .clamp_to_range(false)
                        .text("Rotation axis z height offset"),
                )
                .changed();

            ui.allocate_ui_with_layout(
                Vec2::splat(0.),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label("Side A colour");
                    config_changed |= ui.color_edit_button_srgb(&mut self.side_a_colour).changed();
                },
            );

            ui.allocate_ui_with_layout(
                Vec2::splat(0.),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label("Side B colour");
                    config_changed |= ui.color_edit_button_srgb(&mut self.side_b_colour).changed();
                },
            );

            if ui.button("Reset to defaults").clicked() {
                *self = Self::default();
                config_changed = true;
            }

            config_changed
        }
    }
}

#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;

    /// Spin a split plane around a point in the center of the tree.
    #[derive(Clone, Debug, PartialEq, BaseEffect)]
    pub struct SplitPlane {
        /// The config for this effect.
        config: SplitPlaneConfig,
    }

    impl Default for SplitPlane {
        fn default() -> Self {
            Self {
                config: SplitPlaneConfig::default(),
            }
        }
    }

    impl Effect for SplitPlane {
        type Config = SplitPlaneConfig;

        async fn run(mut self, driver: &mut dyn Driver) {
            driver.display_frame(FrameType::Frame3D(Frame3D::new(
                vec![FrameObject {
                    object: Object::SplitPlane {
                        normal: Vec3::splat(1.).normalize(),
                        k: 1.,
                        positive_side_colour: self.config.side_a_colour,
                        negative_side_colour: self.config.side_b_colour,
                    },
                    colour: [0; 3],
                    fadeoff: 0.,
                }],
                false,
            )));

            sleep!(Duration::from_secs(5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};

    #[tokio::test]
    #[ignore = "SplitPlane is not properly implemented yet"]
    async fn split_plane_test() {
        let mut driver = TestDriver::new(10);
        SplitPlane::default().run(&mut driver).await;

        insta::assert_ron_snapshot!(driver.data);
    }
}

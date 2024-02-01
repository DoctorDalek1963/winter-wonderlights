//! This module contains the [`SplitPlane`] effect.

#[cfg(feature = "config-impls")]
pub use config::SplitPlaneConfig;

#[cfg(feature = "effect-impls")]
pub use effect::SplitPlane;

use crate::effects::prelude::*;

/// Contains the config for the [`SplitPlane`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`SplitPlane`] effect.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct SplitPlaneConfig {
        /// The colour of one side of the split plane.
        pub side_a_colour: RGBArray,

        /// The colour of the other side of the split plane.
        pub side_b_colour: RGBArray,

        /// The level of blending between the two colours.
        ///
        /// See [`ww_frame::object::Object::SplitPlane::blend`](../../../../../ww_frame/object/enum.Object.html#variant.SplitPlane.field.blend).
        pub colour_blend: f32,

        /// The speed of rotation around the axis of rotation, measured in radians per second in
        /// the anti-clockwise direction.
        pub rotation_speed: f32,

        /// The axis of rotation starts as the x-axis, centered vertically in the middle of the
        /// tree. This variable rotates that axis of rotation about the z-axis. Measured in
        /// degrees.
        pub rotation_axis_z_rotation_degrees: f32,

        /// The rotation axis is always parallel to the floor. When this value is 0, the middle of
        /// the rotation axis oscillation will be vertically in the middle of the tree. Positive
        /// values move the middle point up and negative values move it down. Measured in GIFT
        /// coordinate space, so a distance of 1 is the radius of the base of the tree.
        ///
        /// This value has no effect when the vertical oscillation period is 0.
        pub rotation_axis_initial_height_offset: f32,

        /// The vertical movement speed of the rotation axis, measured in GIFT coordinate units
        /// per second.
        pub rotation_axis_vertical_oscillation_speed: f32,
    }

    impl Default for SplitPlaneConfig {
        fn default() -> Self {
            Self {
                side_a_colour: [244, 29, 9],
                side_b_colour: [26, 234, 23],
                colour_blend: 0.05,
                rotation_speed: 0.5,
                rotation_axis_z_rotation_degrees: 0.,
                rotation_axis_initial_height_offset: 0.,
                rotation_axis_vertical_oscillation_speed: 0.5,
            }
        }
    }

    impl EffectConfig for SplitPlaneConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            let mut config_changed = false;

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.colour_blend, 0.0..=0.2)
                        .clamp_to_range(false)
                        .text("Colour blend"),
                )
                .changed();
            if self.colour_blend < 0. {
                self.colour_blend = 0.;
            }

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
                    egui::Slider::new(&mut self.rotation_axis_initial_height_offset, -2.0..=2.0)
                        .clamp_to_range(false)
                        .text("Rotation axis initial height offset"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.rotation_axis_vertical_oscillation_speed, 0.0..=2.)
                        .suffix("units/s")
                        .clamp_to_range(false)
                        .text("Rotation axis vertical oscillation speed"),
                )
                .changed();

            ui.add_space(UI_SPACING);

            config_changed |= colour_picker(ui, &mut self.side_a_colour, "Side A colour").changed();
            config_changed |= colour_picker(ui, &mut self.side_b_colour, "Side B colour").changed();

            config_changed
        }
    }
}

/// Contains the [`SplitPlane`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use std::f32::consts::TAU;

    use super::*;
    use ww_gift_coords::COORDS;

    /// Spin a split plane around a point in the center of the tree.
    #[derive(Clone, Debug, PartialEq, BaseEffect)]
    pub struct SplitPlane {
        /// The height of the center of the plane.
        height: f32,

        /// The current angle of the rotation. Must be between 0 and [`std::f32::consts::TAU`].
        angle: f32,

        /// Are we currently going up or down?
        ///
        /// This has no effect if [`SplitPlaneConfig::rotation_axis_vertical_oscillation_speed`]
        /// is 0.
        going_up: bool,
    }

    impl Effect for SplitPlane {
        fn from_config(config: SplitPlaneConfig) -> Self {
            Self {
                height: COORDS.max_z() / 2. + config.rotation_axis_initial_height_offset,
                angle: 0.,
                going_up: true,
            }
        }

        fn next_frame(&mut self, config: &SplitPlaneConfig) -> Option<(FrameType, Duration)> {
            let normal =
                Quat::from_rotation_z(config.rotation_axis_z_rotation_degrees.to_radians())
                    * Quat::from_rotation_x(self.angle)
                    * Vec3::Y;

            let frame = FrameType::Frame3D(Frame3D::new(
                vec![FrameObject {
                    object: Object::SplitPlane {
                        normal,
                        k: normal.dot(Vec3::new(0., 0., self.height)),
                        blend: config.colour_blend,
                        positive_side_colour: config.side_a_colour,
                        negative_side_colour: config.side_b_colour,
                    },
                    colour: [0; 3],
                    fadeoff: 0.,
                }],
                false,
            ));

            // Update height
            let delta = config.rotation_axis_vertical_oscillation_speed / 50.;

            if self.going_up {
                self.height += delta;
                let limit = COORDS.max_z() * 0.9;

                if self.height > limit {
                    self.height = limit;
                    self.going_up = false;
                }
            } else {
                self.height -= delta;
                let limit = COORDS.max_z() * 0.1;

                if self.height < limit {
                    self.height = limit;
                    self.going_up = true;
                }
            }

            // Update angle
            self.angle += config.rotation_speed / 50.;
            if self.angle > TAU {
                self.angle = 0.;
            }

            Some((frame, Duration::from_millis(20)))
        }

        #[cfg(any(test, feature = "bench"))]
        fn loops_to_test() -> Option<NonZeroU16> {
            NonZeroU16::new(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot_effect;

    #[test]
    fn split_plane_test() {
        snapshot_effect!(SplitPlane);
    }
}

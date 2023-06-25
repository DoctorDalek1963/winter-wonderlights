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
        pub rotation_axis_height_center_offset: f32,

        /// The number of seconds taken for a full vertical oscillation of the rotation axis.
        ///
        /// A negative period should just make it oscillate in reverse, since we're using sin, but
        /// that behaviour should not be relied on.
        pub rotation_axis_vertical_oscillation_period: f32,
    }

    impl Default for SplitPlaneConfig {
        fn default() -> Self {
            Self {
                side_a_colour: [244, 29, 9],
                side_b_colour: [26, 234, 23],
                colour_blend: 0.05,
                rotation_speed: 0.5,
                rotation_axis_z_rotation_degrees: 0.,
                rotation_axis_height_center_offset: 0.,
                rotation_axis_vertical_oscillation_period: 30.,
            }
        }
    }

    impl EffectConfig for SplitPlaneConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            ui.label(RichText::new("SplitPlane config").heading());
            ui.add_space(UI_SPACING);

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
                    egui::Slider::new(&mut self.rotation_axis_height_center_offset, -2.0..=2.0)
                        .clamp_to_range(false)
                        .text("Rotation axis height center offset"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(
                        &mut self.rotation_axis_vertical_oscillation_period,
                        0.0..=60.,
                    )
                    .suffix("s")
                    .clamp_to_range(false)
                    .text("Rotation axis vertical oscillation period"),
                )
                .changed();

            ui.add_space(UI_SPACING);

            config_changed |= colour_picker(ui, &mut self.side_a_colour, "Side A colour").changed();
            config_changed |= colour_picker(ui, &mut self.side_b_colour, "Side B colour").changed();

            ui.add_space(UI_SPACING);

            if ui.button("Reset to defaults").clicked() {
                *self = Self::default();
                config_changed = true;
            }

            config_changed
        }
    }
}

/// Contains the [`SplitPlane`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use ww_gift_coords::COORDS;

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
        #[allow(
            clippy::semicolon_if_nothing_returned,
            reason = "this is a bodge for #[end_loop_in_test_or_bench]"
        )]
        async fn run(self, driver: &mut dyn Driver) {
            let middle_point = Vec3::new(
                0.,
                0.,
                COORDS.max_z() / 2. + self.config.rotation_axis_height_center_offset,
            );

            let mut oscillation_base: f32 = 0.0;
            let max_oscillation_offset = 0.9 * (COORDS.max_z() / 2.);

            let rotation_axis: Vec3 =
                (Quat::from_rotation_z(self.config.rotation_axis_z_rotation_degrees.to_radians())
                    * Vec3::X)
                    .normalize();

            let mut normal = Vec3::Z;

            #[end_loop_in_test_or_bench]
            loop {
                let point_on_plane = if self.config.rotation_axis_vertical_oscillation_period != 0.
                {
                    let sin = oscillation_base.sin();
                    middle_point
                        + Vec3::new(0., 0., max_oscillation_offset * sin * sin.abs().sqrt())
                } else {
                    middle_point
                };

                driver.display_frame(FrameType::Frame3D(Frame3D::new(
                    vec![FrameObject {
                        object: Object::SplitPlane {
                            normal,
                            k: normal.dot(point_on_plane),
                            blend: self.config.colour_blend,
                            positive_side_colour: self.config.side_a_colour,
                            negative_side_colour: self.config.side_b_colour,
                        },
                        colour: [0; 3],
                        fadeoff: 0.,
                    }],
                    false,
                )));

                if self.config.rotation_axis_vertical_oscillation_period != 0. {
                    oscillation_base += std::f32::consts::TAU
                        / self.config.rotation_axis_vertical_oscillation_period
                        / 100.;
                }

                normal = Quat::from_axis_angle(rotation_axis, self.config.rotation_speed / 100.)
                    * normal;
                trace!(?normal);
                //trace!(length = normal.clone().length());

                sleep!(Duration::from_millis(10));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};

    #[tokio::test]
    async fn split_plane_test() {
        let mut driver = TestDriver::new(10);
        SplitPlane::default().run(&mut driver).await;

        insta::assert_ron_snapshot!(driver.data);
    }
}

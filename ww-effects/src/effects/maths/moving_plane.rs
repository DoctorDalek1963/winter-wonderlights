//! This module contains the [`MovingPlane`] effect.

#[cfg(feature = "config-impls")]
pub use config::MovingPlaneConfig;

#[cfg(feature = "effect-impls")]
pub use effect::MovingPlane;

use crate::effects::prelude::*;

/// Contains the config for the [`MovingPlane`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`MovingPlane`] effect; includes speed.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct MovingPlaneConfig {
        /// How many units (in GIFT coords) that the plane moves in one second.
        pub units_per_second: f32,

        /// The thickness of the plane.
        pub thickness: f32,

        /// The maximum distance where colour drops to zero.
        ///
        /// See [`ww_frame::FrameObject::fadeoff`].
        pub fadeoff: f32,
    }

    impl Default for MovingPlaneConfig {
        fn default() -> Self {
            Self {
                units_per_second: 0.1,
                thickness: 0.1,
                fadeoff: 0.08,
            }
        }
    }

    impl EffectConfig for MovingPlaneConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            let mut config_changed = false;

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.units_per_second, 0.0..=1.0)
                        .suffix("/s")
                        .text("Speed (units/s)"),
                )
                .changed();

            config_changed |= ui
                .add(egui::Slider::new(&mut self.thickness, 0.0..=0.5).text("Thickness"))
                .changed();

            config_changed |= ui
                .add(egui::Slider::new(&mut self.fadeoff, 0.0..=0.25).text("Fadeoff"))
                .changed();

            config_changed
        }
    }
}

/// Contains the [`MovingPlane`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use ww_gift_coords::COORDS;

    /// Move a plane through the tree at a random angle with a random colour.
    #[derive(Clone, Debug, PartialEq, BaseEffect)]
    pub struct MovingPlane {
        /// The colour of this plane.
        colour: RGBArray,

        /// The direction that this plane is travelling in.
        normal_vector: Vec3,

        /// The position of the center of the plane.
        position: Vec3,

        /// Are we in the start phase?
        ///
        /// This value starts by being true. Once we generate a frame where the plane lights up at
        /// least one light, this is set to false. Once it's false, we start checking if the frame
        /// has no lights lit up, and if that's true, we stop.
        in_start_phase: bool,
    }

    /// Generate one frame of the effect by creating a plane intersecting the given point with the
    /// given normal vector and config.
    fn generate_frame(
        point: Vec3,
        colour: RGBArray,
        normal: Vec3,
        config: &MovingPlaneConfig,
    ) -> Frame3D {
        Frame3D::new(
            vec![FrameObject {
                object: Object::Plane {
                    normal,
                    k: normal.dot(point),
                    threshold: config.thickness,
                },
                colour,
                fadeoff: config.fadeoff,
            }],
            false,
        )
    }

    impl Effect for MovingPlane {
        fn from_config(config: MovingPlaneConfig) -> Self {
            // TODO: Add this to user-accessible config?
            /// The proportion of the length of the normal vector that we move the center point
            /// back by until it's outside of the tree.
            const MOVE_PROPORTION: f32 = 0.1;

            let mut rng = rng!();
            let colour = rng.gen();
            let normal_vector = random_vector(&mut rng);

            //// Start in the middle and reverse with normal vector until outside bounding box
            let mut position: Vec3 = COORDS.center().into();
            while COORDS.distance_from_bounding_box(position.into()) <= 0. {
                position -= normal_vector * MOVE_PROPORTION;
            }

            let mut frame = generate_frame(position, colour, normal_vector, &config);

            // While there are any non-black lights, keep moving the point out
            while let Some(data) = frame.compute_raw_data().raw_data()
                && data.iter().any(|colour| *colour != [0; 3])
            {
                position -= normal_vector * MOVE_PROPORTION;
                frame = generate_frame(position, colour, normal_vector, &config);
            }
            position += normal_vector * MOVE_PROPORTION;

            Self {
                colour,
                normal_vector,
                position,
                in_start_phase: true,
            }
        }

        fn next_frame(&mut self, config: &MovingPlaneConfig) -> Option<(FrameType, Duration)> {
            let mut frame = generate_frame(self.position, self.colour, self.normal_vector, config);

            frame.compute_raw_data();
            let all_lights_are_off = frame
                .raw_data()
                .expect_or_log("We've already called compute_raw_data()")
                .iter()
                .all(|colour| colour == &[0; 3]);

            // We're going to sleep for 20ms every loop, which gives 50 fps. This means we
            // want to move 1/50th of the units per second
            self.position += (config.units_per_second / 50.) * self.normal_vector;

            if self.in_start_phase && !all_lights_are_off {
                self.in_start_phase = false;
            }

            if !self.in_start_phase && all_lights_are_off {
                return None;
            }

            Some((FrameType::Frame3D(frame), Duration::from_millis(20)))
        }

        #[cfg(any(test, feature = "bench"))]
        fn loops_to_test() -> Option<NonZeroU16> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot_effect;

    #[test]
    fn moving_plane_test() {
        snapshot_effect!(MovingPlane);
    }
}

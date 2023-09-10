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
        /// The config for this effect.
        config: MovingPlaneConfig,

        /// The RNG generator to use for randomness.
        ///
        /// This is seeded with a known value for testing purposes.
        rng: StdRng,
    }

    impl Effect for MovingPlane {
        fn from_config(config: MovingPlaneConfig) -> Self {
            Self {
                config,
                rng: rng!(),
            }
        }

        async fn run(mut self, driver: &mut dyn Driver) {
            let colour: RGBArray = self.rng.gen();
            let normal: Vec3 = random_vector(&mut self.rng);

            let threshold = self.config.thickness;
            let fadeoff = self.config.fadeoff;

            let get_frame = |point| {
                Frame3D::new(
                    vec![FrameObject {
                        object: Object::Plane {
                            normal,
                            k: normal.dot(point),
                            threshold,
                        },
                        colour,
                        fadeoff,
                    }],
                    false,
                )
            };

            let (mut point, mut frame): (Vec3, Frame3D) = {
                /// The proportion of the length of the normal vector that we move the center point
                /// back by until it's outside of the tree.
                const MOVE_PROPORTION: f32 = 0.1;

                // Start in the middle and reverse with normal vector until outside bounding box
                let mut p: Vec3 = COORDS.center().into();
                while COORDS.distance_from_bounding_box(p.into()) <= 0. {
                    p -= normal * MOVE_PROPORTION;
                }

                let mut frame = get_frame(p);

                // While there are any non-black lights, keep moving the point out
                while let Some(data) = frame .compute_raw_data().raw_data()
                    && data
                        .iter()
                        .any(|colour| *colour != [0; 3])
                {
                    p -= normal * MOVE_PROPORTION;
                    frame = get_frame(p);
                }
                p += normal * MOVE_PROPORTION;

                (p, frame)
            };

            /// Display a single frame, update the mutable variables for the next frame, then sleep.
            macro_rules! do_frame {
                () => {
                    driver.display_frame(FrameType::Frame3D(frame));

                    // We're going to sleep for 20ms every loop, which gives 50 fps. This means we
                    // want to move 1/50th of the units per second
                    point += (self.config.units_per_second / 50.) * normal;
                    frame = get_frame(point);
                    sleep!(Duration::from_millis(20));
                };
            }

            // Do one frame first to light up some of the lights
            do_frame!();

            // While not all lights are black, keep moving
            while let Some(data) = frame.compute_raw_data().raw_data()
                && !data
                    .iter()
                    .all(|colour| *colour == [0; 3])
            {
                do_frame!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};

    #[tokio::test]
    async fn moving_plane_test() {
        let mut driver = TestDriver::new(10);
        MovingPlane::default().run(&mut driver).await;

        // The plane moves through the whole tree, and that results in thousands of individual
        // frames, which is far too many to inline here
        insta::assert_ron_snapshot!(driver.data);
    }
}

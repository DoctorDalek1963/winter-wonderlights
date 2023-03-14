//! This module contains effects that deal with planes.

use crate::{sleep, Effect, EffectConfig};
use async_trait::async_trait;
use egui::RichText;
use glam::Vec3;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use ww_driver_trait::Driver;
use ww_frame::{random_vector, Frame3D, FrameObject, FrameType, Object, RGBArray};
use ww_gift_coords::COORDS;

/// The config for the moving plane effect; includes speed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovingPlaneConfig {
    /// How many units (in GIFT coords) that the plane moves in one second.
    units_per_second: f32,

    /// The thickness of the plane.
    thickness: f32,

    /// The maximum distance where colour drops to zero.
    ///
    /// See [`ww_frame::FrameObject::fadeoff`].
    fadeoff: f32,
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
    fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.separator();
        ui.label(RichText::new("MovingPlane config").heading());

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

        if ui.button("Reset to defaults").clicked() {
            *self = Self::default();
            config_changed = true;
        }

        if config_changed {
            self.save_to_file(&MovingPlane::config_filename());
        }
    }
}

/// Move a plane through the tree at a random angle with a random colour.
pub struct MovingPlane {
    /// The config for this effect.
    config: MovingPlaneConfig,

    /// The RNG generator to use for randomness.
    ///
    /// This is seeded with a known value for testing purposes.
    rng: StdRng,
}

impl Default for MovingPlane {
    #[cfg(test)]
    fn default() -> Self {
        Self {
            config: MovingPlaneConfig::default(),
            rng: StdRng::seed_from_u64(12345),
        }
    }

    #[cfg(not(test))]
    fn default() -> Self {
        Self {
            config: MovingPlaneConfig::default(),
            rng: StdRng::from_entropy(),
        }
    }
}

#[async_trait]
impl Effect for MovingPlane {
    type Config = MovingPlaneConfig;

    fn effect_name() -> &'static str
    where
        Self: Sized,
    {
        "MovingPlane"
    }

    async fn run(mut self, driver: &mut dyn Driver) {
        let colour: RGBArray = self.rng.gen();
        let normal: Vec3 = random_vector(&mut self.rng);

        let threshold = self.config.thickness;
        let fadeoff = self.config.fadeoff;
        let dist_from_bb = 1.3 * (threshold + fadeoff);

        // Start in the middle and reverse with normal vector until outside bounding box
        let mut point: Vec3 = {
            let mut p: Vec3 = COORDS.center().into();
            while COORDS.distance_from_bounding_box(p.into()) < dist_from_bb {
                p -= normal * 0.1;
            }
            p + normal * 0.1
        };

        while COORDS.distance_from_bounding_box(point.into()) < dist_from_bb {
            driver.display_frame(FrameType::Frame3D(Frame3D {
                objects: vec![FrameObject {
                    object: Object::Plane {
                        normal,
                        k: normal.dot(point),
                        threshold,
                    },
                    colour,
                    fadeoff,
                }],
                blend: false,
            }));

            // We're going to sleep for 20ms every loop, which gives 50 fps. This means we want to
            // move 1/50th of the units per second
            point += (self.config.units_per_second / 50.) * normal;
            sleep!(Duration::from_millis(20));
        }
    }

    fn save_to_file(&self) {
        self.config.save_to_file(&Self::config_filename());
    }

    fn from_file() -> Self {
        Self {
            config: Self::Config::from_file(&Self::config_filename()),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestDriver;

    #[tokio::test]
    async fn moving_plane_test() {
        let mut driver = TestDriver::new(10);
        MovingPlane::default().run(&mut driver).await;

        // The frame moves through the whole tree, and that results in thousands of individual
        // frames, which is far too many to inline here
        insta::assert_ron_snapshot!(driver.data);
    }
}

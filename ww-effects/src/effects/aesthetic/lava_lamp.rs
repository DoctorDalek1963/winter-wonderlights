//! This module contains the lava lamp effect.

use crate::{sleep, Effect, EffectConfig};
use async_trait::async_trait;
use effect_proc_macros::{BaseEffect, Sealed};
use egui::{Align, Layout, RichText, Vec2};
use glam::{IVec3, Vec3};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument};
use ww_driver_trait::Driver;
use ww_frame::{random_vector, Frame3D, FrameObject, FrameType, Object, RGBArray};
use ww_gift_coords::COORDS;

/// A simple sphere used to keep track of the spheres in the lava lamp.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sphere {
    /// The position of the centre of the sphere.
    centre: Vec3,

    // The radius of the sphere.
    radius: f32,

    /// The colour offset of this sphere. Added to the base colour to get the colour of the sphere.
    colour_offset: IVec3,

    movement_direction: Vec3,
}

impl Sphere {
    /// Compute the colour of the sphere from its colour offset and the given base colour.
    fn get_colour(&self, base_colour: RGBArray) -> RGBArray {
        let [r, g, b] = base_colour;
        [
            (r as i32 + self.colour_offset.x).clamp(0, 255) as u8,
            (g as i32 + self.colour_offset.y).clamp(0, 255) as u8,
            (b as i32 + self.colour_offset.z).clamp(0, 255) as u8,
        ]
    }

    /// Create a frame object from the sphere.
    fn into_frame_object(self, base_colour: RGBArray, fadeoff: f32) -> FrameObject {
        FrameObject {
            object: Object::Sphere {
                center: self.centre,
                radius: self.radius,
            },
            colour: self.get_colour(base_colour),
            fadeoff,
        }
    }
}

/// The config for the lava lamp effect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Sealed)]
pub struct LavaLampConfig {
    /// The base colour of the spheres.
    base_colour: RGBArray,

    /// The maximum RBG colour variation from the base.
    variation: u8,

    /// The maximum distance where colour drops to zero.
    ///
    /// See [`ww_frame::FrameObject::fadeoff`].
    fadeoff: f32,
}

impl Default for LavaLampConfig {
    fn default() -> Self {
        Self {
            base_colour: [243, 83, 255],
            variation: 20,
            fadeoff: 0.3,
        }
    }
}

impl EffectConfig for LavaLampConfig {
    fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.separator();
        ui.label(RichText::new("LavaLamp config").heading());

        let mut config_changed = false;

        config_changed |= ui
            .add(egui::Slider::new(&mut self.fadeoff, 0.0..=1.5).text("Fadeoff"))
            .changed();
        config_changed |= ui
            .add(egui::Slider::new(&mut self.variation, 1..=255).text("Colour variation"))
            .changed();

        ui.allocate_ui_with_layout(
            Vec2::splat(0.),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.label("Base colour: ");
                config_changed |= ui.color_edit_button_srgb(&mut self.base_colour).changed();
            },
        );

        if ui.button("Reset to defaults").clicked() {
            *self = Self::default();
            config_changed = true;
        }

        if config_changed {
            self.save_to_file(&LavaLamp::config_filename());
        }
    }
}

/// Display a lava lamp-like effect on the tree.
#[derive(Clone, Debug, PartialEq, BaseEffect)]
pub struct LavaLamp {
    /// The config for this effect.
    config: LavaLampConfig,

    /// The RNG generator to use for randomness.
    ///
    /// This is seeded with a known value for testing purposes.
    rng: StdRng,
}

impl Default for LavaLamp {
    #[cfg(test)]
    fn default() -> Self {
        Self {
            config: LavaLampConfig::default(),
            rng: StdRng::seed_from_u64(12345),
        }
    }

    #[cfg(not(test))]
    fn default() -> Self {
        Self {
            config: LavaLampConfig::default(),
            rng: StdRng::from_entropy(),
        }
    }
}

#[async_trait]
impl Effect for LavaLamp {
    type Config = LavaLampConfig;

    #[instrument(skip_all)]
    async fn run(mut self, driver: &mut dyn Driver) {
        // Spawn some spheres (number in config?) and gradually change their sizes and colours over
        // time while moving them all up and down at random speeds

        let mut spheres: Vec<Sphere> = vec![];
        for _ in 0..5 {
            spheres.push(Sphere {
                centre: Vec3 {
                    x: self.rng.gen_range(-1.0..1.0),
                    y: self.rng.gen_range(-1.0..1.0),
                    z: self.rng.gen_range(0.0..COORDS.max_z()),
                },
                radius: self.rng.gen_range(0.25..2.0),
                colour_offset: {
                    let range = -(self.config.variation as i32)..(self.config.variation as i32);
                    IVec3 {
                        x: self.rng.gen_range(range.clone()),
                        y: self.rng.gen_range(range.clone()),
                        z: self.rng.gen_range(range),
                    }
                },
                movement_direction: random_vector(&mut self.rng),
            });
        }
        debug!(?spheres);

        #[cfg(any(test, feature = "bench"))]
        let mut counter: u8 = 0;

        loop {
            let sphere_frame_objects = spheres
                .iter()
                .map(|&sphere| {
                    sphere.into_frame_object(self.config.base_colour, self.config.fadeoff)
                })
                .collect();

            driver.display_frame(FrameType::Frame3D(Frame3D {
                objects: sphere_frame_objects,
                blend: true,
            }));

            for sphere in spheres.iter_mut() {
                sphere.centre += 0.05 * sphere.movement_direction;
                sphere.movement_direction =
                    (sphere.movement_direction + 0.01 * random_vector(&mut self.rng)).normalize();

                while !COORDS
                    .is_within_bounding_box((sphere.centre + sphere.movement_direction).into())
                {
                    sphere.movement_direction =
                        (sphere.movement_direction + random_vector(&mut self.rng)).normalize();
                }
            }

            sleep!(Duration::from_millis(100));

            #[cfg(any(test, feature = "bench"))]
            {
                counter += 1;
                if counter > 100 {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestDriver;

    #[tokio::test]
    async fn lava_lamp_test() {
        let mut driver = TestDriver::new(10);
        LavaLamp::default().run(&mut driver).await;

        insta::assert_ron_snapshot!(driver.data);
    }
}

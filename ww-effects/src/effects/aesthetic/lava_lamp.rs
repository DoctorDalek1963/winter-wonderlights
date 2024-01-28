//! This module contains the lava lamp effect.

#[cfg(feature = "config-impls")]
pub use config::LavaLampConfig;

#[cfg(feature = "effect-impls")]
pub use effect::LavaLamp;

use crate::effects::prelude::*;

/// Contains the config for the [`LavaLamp`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`LavaLamp`] effect.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct LavaLampConfig {
        /// The base colour of the spheres.
        pub base_colour: [u8; 3],

        /// The maximum RBG colour variation from the base.
        pub variation: u8,

        /// The maximum distance where colour drops to zero.
        ///
        /// See [`ww_frame::FrameObject::fadeoff`].
        pub fadeoff: f32,
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
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            let mut config_changed = false;

            config_changed |= ui
                .add(egui::Slider::new(&mut self.fadeoff, 0.0..=1.5).text("Fadeoff"))
                .changed();

            config_changed |= ui
                .add(egui::Slider::new(&mut self.variation, 1..=255).text("Colour variation"))
                .changed();

            ui.add_space(UI_SPACING);

            config_changed |= colour_picker(ui, &mut self.base_colour, "Base colour").changed();

            config_changed
        }
    }
}

/// Contains the [`LavaLamp`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use glam::IVec3;
    use ww_gift_coords::COORDS;

    /// A simple sphere used to keep track of the spheres in the lava lamp.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Sphere {
        /// The position of the centre of the sphere.
        centre: Vec3,

        /// The radius of the sphere.
        radius: f32,

        /// The colour offset of this sphere. Added to the base colour to get the colour of the sphere.
        colour_offset: IVec3,

        /// The direction that the sphere is currently moving in.
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
        fn into_frame_object(&self, base_colour: RGBArray, fadeoff: f32) -> FrameObject {
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

    /// Display a lava lamp-like effect on the tree.
    #[derive(Clone, Debug, PartialEq, BaseEffect)]
    pub struct LavaLamp {
        /// The RNG used to move the spheres randomly.
        rng: StdRng,

        /// The spheres themselves.
        spheres: Vec<Sphere>,
    }

    impl Effect for LavaLamp {
        fn from_config(config: LavaLampConfig) -> Self {
            let mut rng = rng!();

            let spheres = (0..5)
                .map(|_| Sphere {
                    centre: Vec3 {
                        x: rng.gen_range(-1.0..1.0),
                        y: rng.gen_range(-1.0..1.0),
                        z: rng.gen_range(0.0..COORDS.max_z()),
                    },
                    radius: rng.gen_range(0.25..2.0),
                    colour_offset: {
                        let range = -(config.variation as i32)..(config.variation as i32);
                        IVec3 {
                            x: rng.gen_range(range.clone()),
                            y: rng.gen_range(range.clone()),
                            z: rng.gen_range(range),
                        }
                    },
                    movement_direction: random_vector(&mut rng),
                })
                .collect();
            trace!(?spheres);

            Self { rng, spheres }
        }

        fn next_frame(&mut self, config: &LavaLampConfig) -> Option<(FrameType, Duration)> {
            let sphere_frame_objects = self
                .spheres
                .iter()
                .map(|&sphere| sphere.into_frame_object(config.base_colour, config.fadeoff))
                .collect();

            let frame = FrameType::Frame3D(Frame3D::new(sphere_frame_objects, true));

            for sphere in &mut self.spheres {
                sphere.centre += 0.05 * sphere.movement_direction;
                sphere.movement_direction =
                    (sphere.movement_direction + 0.01 * random_vector(&mut self.rng)).normalize();

                while !COORDS
                    .is_within_bounding_box((sphere.centre + sphere.movement_direction).into())
                {
                    // We're not multiplying the random vector by 0.01 here because if we did, then
                    // this loop would take ages to bring the spheres back into the bounding box
                    sphere.movement_direction =
                        (sphere.movement_direction + random_vector(&mut self.rng)).normalize();
                }
            }

            Some((frame, Duration::from_millis(100)))
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
    fn lava_lamp_test() {
        snapshot_effect!(LavaLamp);
    }
}

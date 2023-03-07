//! This module handles the inidividual objects in frames.

use super::RGBArray;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};
use ww_gift_coords::COORDS;

/// A single object in the frame, with associated colour and fadeoff.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameObject {
    /// The object itself.
    pub object: Object,

    /// The colour of the object.
    pub colour: RGBArray,

    /// The maximum distance from the object where the colour still appears.
    ///
    /// If this is 0, then there is no fadeoff and the border is hard. Otherwise, the contained
    /// value is the distance at which the colour drops to zero. Any distance from the object less
    /// than that will have a colour faded to the correct level.
    ///
    /// A negative fadeoff doesn't make any sense.
    pub fadeoff: f32,
}

impl FrameObject {
    #[instrument(skip(data))]
    pub(super) fn render_into_slice(&self, data: &mut [RGBArray]) {
        match self.object {
            Object::Plane {
                normal,
                k,
                threshold,
            } => {
                for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                    // Get the distance from this point to the plane
                    let dist = f32::abs(normal.dot(point.into()) - k) / normal.length();
                    assert!(
                        dist >= 0.,
                        "Distance from the point to the plane should never be negative"
                    );
                    trace!(?point, ?dist, "Distance from point to plane");

                    // If distance is less than the threshold, then it's part of the plane
                    if dist <= threshold {
                        *light_colour = self.colour;

                    // If the distance is between the threshold and the fadeoff, then it must be
                    // coloured accordingly
                    } else if dist > threshold && dist <= threshold + self.fadeoff {
                        self.set_light_colour_by_fade(dist - threshold, light_colour);
                    }
                }
            }

            Object::Sphere { center, radius } => {
                for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                    // Distance from point to center
                    let dist = {
                        let dx = point.0 - center.x;
                        let dy = point.1 - center.y;
                        let dz = point.2 - center.z;
                        f32::sqrt(dx * dx + dy * dy + dz * dz)
                    };
                    trace!(?point, ?dist, "Distance from point to center");

                    if dist <= radius {
                        *light_colour = self.colour;
                    } else if dist > radius && dist <= radius + self.fadeoff {
                        self.set_light_colour_by_fade(dist - radius, light_colour);
                    }
                }
            }
        }
    }

    /// Work out the amount to fade (in [0, 1]) given a distance into the fade zone.
    fn get_fade(&self, distance: f32) -> f32 {
        (1. - distance / self.fadeoff).clamp(0., 1.)
    }

    /// Set the light colour according to the appropriate fade off, given a distance into the fade
    /// zone and a mut reference to the colour to be changed.
    fn set_light_colour_by_fade(&self, distance: f32, light_colour: &mut RGBArray) {
        let fade = self.get_fade(distance);
        let [r, g, b] = self.colour;
        *light_colour = [
            (r as f32 * fade) as u8,
            (g as f32 * fade) as u8,
            (b as f32 * fade) as u8,
        ];
    }
}

/// An object in a 3D frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    /// A plane defined in terms of normal vector and dot product result.
    Plane {
        /// The normal vector of the plane.
        normal: Vec3,

        /// The result of dotting the direction vector with any point on the plane.
        k: f32,

        /// The maximum distance from this object where lights will be counted as part of the
        /// object.
        threshold: f32,
    },

    /// A sphere with a center and radius.
    Sphere {
        /// The coordinates of the center of the sphere.
        center: Vec3,

        /// The radius of the sphere.
        radius: f32,
    },
}

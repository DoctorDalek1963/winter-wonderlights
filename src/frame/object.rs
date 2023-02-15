//! This module handles the inidividual objects in frames.

use super::RGBArray;
use crate::gift_coords::COORDS;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};

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
    #[instrument(skip_all)]
    pub(super) fn render_into_vec(&self, data: &mut [RGBArray]) {
        match self.object {
            Object::Plane {
                normal,
                k,
                threshold,
            } => {
                for (light, &point) in data.iter_mut().zip(COORDS.coords()) {
                    // Get the distance from this point to the plane
                    let dist = f32::abs(normal.dot(point.into()) - k) / normal.length();
                    assert!(
                        dist >= 0.,
                        "Distance from the point to the plane should never be negative"
                    );
                    trace!(?point, ?dist, "Distance from point to plane");

                    // If distance is less than the threshold, then it's part of the plane
                    if dist <= threshold {
                        *light = self.colour;

                    // If the distance is between the threshold and the fadeoff, then it must be
                    // coloured accordingly
                    } else if dist > threshold && dist <= threshold + self.fadeoff {
                        let fade = 1. - (dist - threshold) / self.fadeoff;
                        assert!(fade >= 0. && fade <= 1., "Fade should always be in [0, 1]");

                        let [r, g, b] = self.colour;
                        *light = [
                            (r as f32 * fade) as u8,
                            (g as f32 * fade) as u8,
                            (b as f32 * fade) as u8,
                        ];
                    }
                }
            }
        }
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
}

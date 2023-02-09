//! This module provides functionality for specifiying and using 3D frames.

use crate::{gift_coords::COORDS, vecs::Vec3};
use tracing::{debug, instrument};

/// An RGB colour.
pub type RGBArray = [u8; 3];

/// A type of frame data.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameType {
    /// A frame indicating that all the lights are off.
    Off,

    /// The colour for each light on the chain.
    ///
    /// The driver is expected to handle the cases where the Vec is too long or too short and
    /// doesn't corespond one-to-one to the lights, but this behaviour is specific to individual
    /// drivers and should not be relied on.
    RawData(Vec<RGBArray>),

    /// A sophisticated 3D frame, made of several objects.
    Frame3D(Frame3D),
}

/// A 3D frame, made of several objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame3D {
    /// The vec of objects in the frame.
    ///
    /// An object later in the vec will override any object previous in the list. No blending of
    /// colours is performed.
    pub objects: Vec<FrameObject>,
}

impl Frame3D {
    /// Convert the frame to a raw data vec, using [`COORDS`] to know where the lights are.
    pub fn to_raw_data(&self) -> Vec<RGBArray> {
        let mut data: Vec<RGBArray> = vec![[0, 0, 0]; COORDS.lights_num()];
        debug!(?data);

        for frame_object in &self.objects {
            frame_object.render_into_vec(&mut data);
        }

        debug!(?data);
        data
    }
}

/// A single object in the frame, with associated colour and fadeoff.
#[derive(Clone, Debug, PartialEq)]
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
    pub fadeoff: f64,
}

impl FrameObject {
    #[instrument(skip_all)]
    fn render_into_vec(&self, data: &mut Vec<RGBArray>) {
        match self.object {
            Object::Plane {
                normal,
                k,
                threshold,
            } => {
                for (light, &point) in data.iter_mut().zip(COORDS.coords()) {
                    // Get the distance from this point to the plane
                    let dist = f64::abs(normal.dot(&point.into()) - k) / normal.length();
                    assert!(
                        dist >= 0.,
                        "Distance from the point to the plane should never be negative"
                    );
                    debug!(?point, ?dist, "Distance from point to plane");

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
                            (r as f64 * fade) as u8,
                            (g as f64 * fade) as u8,
                            (b as f64 * fade) as u8,
                        ];
                    }
                }
            }
        }
    }
}

/// An object in a 3D frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    /// A plane defined in terms of normal vector and dot product result.
    Plane {
        /// The normal vector of the plane.
        normal: Vec3,

        /// The result of dotting the direction vector with any point on the plane.
        k: f64,

        /// The maximum distance from this object where lights will be counted as part of the
        /// object.
        threshold: f64,
    },
}

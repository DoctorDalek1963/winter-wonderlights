//! This module provides functionality for specifiying and using 3D frames.

use crate::gift_coords::COORDS;
use glam::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::trace;

mod object;

pub use self::object::{FrameObject, Object};

/// An RGB colour.
pub type RGBArray = [u8; 3];

/// Generate a random `Vec3` with positive or negative elements, and normalize it.
pub fn random_vector<R: Rng + ?Sized>(rng: &mut R) -> Vec3 {
    (rng.gen::<Vec3>() - Vec3::new(0.5, 0.5, 0.5)).normalize()
}

/// A type of frame data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame3D {
    /// The vec of objects in the frame.
    ///
    /// An object later in the vec will override any object previous in the list. No blending of
    /// colours is performed.
    pub objects: Vec<FrameObject>,

    /// Whether to blend objects together.
    pub blend: bool,
}

impl Frame3D {
    /// Convert the frame to a raw data vec, using [`struct@COORDS`] to know where the lights are.
    pub fn to_raw_data(&self) -> Vec<RGBArray> {
        let mut data: Vec<RGBArray> = vec![[0, 0, 0]; COORDS.lights_num()];
        trace!(?data, "Before");

        if self.blend {
            let total = self.objects.len() as f32;

            data = self
                .objects
                .iter()

                // Render the data into its own slice and convert all the u8s to f32s for later
                .map(|f_obj| {
                    let mut new_data = data.clone();
                    f_obj.render_into_slice(&mut new_data);
                    new_data
                        .into_iter()
                        // Divide by the total here to make the mean easier to calculate
                        .map(|[r, g, b]| [r as f32 / total, g as f32 / total, b as f32 / total])
                        .collect::<Vec<_>>()
                })

                // Sum all the data to get one Vec<[f32; 3]> where each element is the mean colour
                // for that light
                .fold(vec![[0., 0., 0.]; COORDS.lights_num()], |mut acc, v| {
                    for i in 0..COORDS.lights_num() {
                        acc[i][0] += v[i][0];
                        acc[i][1] += v[i][1];
                        acc[i][2] += v[i][2];
                    }
                    acc
                })
                .into_iter()

                // Convert everything back to u8s
                .map(|[r, g, b]| [r as u8, g as u8, b as u8])
                .collect();
        } else {
            for frame_object in &self.objects {
                frame_object.render_into_slice(&mut data);
            }
        }

        trace!(?data, "After");
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_raw_data_test() {
        let green_plane = FrameObject {
            object: Object::Plane {
                normal: Vec3::new(1., 0.5, -3.5).normalize(),
                k: -1.2354,
                threshold: 0.15,
            },
            colour: [25, 200, 16],
            fadeoff: 0.14,
        };

        let single_plane = Frame3D {
            objects: vec![green_plane],
            blend: false,
        };

        insta::with_settings!({
            info => &single_plane,
            description => "Rendering a single plane to raw data with no blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(single_plane.to_raw_data());
        });
    }
}

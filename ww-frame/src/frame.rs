//! This module handles the implementations of the frames.

use crate::{FrameObject, RGBArray};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::trace;
use ww_gift_coords::COORDS;

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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3D {
    /// The vec of objects in the frame.
    ///
    /// An object later in the vec will override any object previous in the list. No blending of
    /// colours is performed.
    objects: Vec<FrameObject>,

    /// Whether to blend objects together.
    blend: bool,

    /// Optionally pre-computed data. If an effect needs to compute the raw data first, then it can
    /// just store that data here so the driver doesn't have to re-compute it later.
    #[cfg_attr(any(feature = "insta", test), serde(skip))]
    pre_computed_raw_data: Option<Vec<RGBArray>>,
}

impl fmt::Debug for Frame3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame3D")
            .field("objects", &self.objects)
            .field("blend", &self.blend)
            //.field("pre_computed_raw_data", &self.pre_computed_raw_data)
            .finish()
    }
}

impl Frame3D {
    /// Create a new frame.
    pub fn new(objects: Vec<FrameObject>, blend: bool) -> Self {
        Self {
            objects,
            blend,
            pre_computed_raw_data: None,
        }
    }

    /// Compute the vec of raw data for this frame, using [`struct@COORDS`] to know where the
    /// lights are. Use [`Self::to_raw_data`] to get the data out or [`Self::raw_data`] to
    /// reference the optional data.
    ///
    /// This function does nothing if the data was already computed.
    ///
    /// This function returns a reference to self to allow things like:
    /// ```
    /// #![feature(let_chains)]
    ///
    /// # use ww_frame::Frame3D;
    /// # fn do_something(_: &Vec<[u8; 3]>) -> bool { true }
    /// # let mut frame = Frame3D::new(vec![], false);
    /// while let Some(data) = frame.compute_raw_data().raw_data() && do_something(data) {
    ///     // Do stuff
    ///     # break;
    /// }
    /// ```
    /// Admittedly this API isn't very ergonomic and this function should ideally return a
    /// reference to the computed data, but the borrow checker didn't like that, so we're currently
    /// using this bodge.
    pub fn compute_raw_data(&mut self) -> &mut Self {
        if self.pre_computed_raw_data.is_some() {
            return self;
        }

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
        self.pre_computed_raw_data = Some(data);
        self
    }

    /// Return the raw data for this frame.
    pub fn to_raw_data(mut self) -> Vec<RGBArray> {
        self.compute_raw_data();
        self.pre_computed_raw_data.expect(
            "pre_computed_raw_data must be populated since compute_raw_data() has been called",
        )
    }

    /// Get an optional reference to the internal pre-computed data.
    pub fn raw_data(&self) -> Option<&Vec<RGBArray>> {
        self.pre_computed_raw_data.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Object;
    use glam::Vec3;

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
        let blue_plane = FrameObject {
            object: Object::Plane {
                normal: Vec3::new(-1.6, 1.99, -0.02).normalize(),
                k: 0.24,
                threshold: 0.3,
            },
            colour: [0, 12, 186],
            fadeoff: 0.82,
        };
        let red_sphere = FrameObject {
            object: Object::Sphere {
                center: Vec3::new(0.01, 0.31, 0.2),
                radius: 1.2,
            },
            colour: [243, 36, 17],
            fadeoff: 0.1,
        };

        // Single green plane

        let single_green_plane = Frame3D::new(vec![green_plane.clone()], false);
        insta::with_settings!({
            info => &single_green_plane,
            description => "Rendering a single plane with no blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(single_green_plane.to_raw_data());
        });

        // Green and blue planes

        let green_and_blue_planes_no_blend =
            Frame3D::new(vec![green_plane.clone(), blue_plane.clone()], false);
        insta::with_settings!({
            info => &green_and_blue_planes_no_blend,
            description => "Rendering two planes with no blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(green_and_blue_planes_no_blend.to_raw_data());
        });

        let green_and_blue_planes_with_blend =
            Frame3D::new(vec![green_plane.clone(), blue_plane.clone()], true);
        insta::with_settings!({
            info => &green_and_blue_planes_with_blend,
            description => "Rendering two planes with with blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(green_and_blue_planes_with_blend.to_raw_data());
        });

        // Blue plane and red sphere

        let blue_plane_red_sphere_no_blend =
            Frame3D::new(vec![blue_plane.clone(), red_sphere.clone()], false);
        insta::with_settings!({
            info => &blue_plane_red_sphere_no_blend,
            description => "Rendering a blue plane and then a red sphere with no blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(blue_plane_red_sphere_no_blend.to_raw_data());
        });

        let red_sphere_blue_plane_no_blend =
            Frame3D::new(vec![red_sphere.clone(), blue_plane.clone()], false);
        insta::with_settings!({
            info => &red_sphere_blue_plane_no_blend,
            description => "Rendering a red sphere and then a blue plane with no blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(red_sphere_blue_plane_no_blend.to_raw_data());
        });

        let blue_plane_red_sphere_with_blend =
            Frame3D::new(vec![blue_plane.clone(), red_sphere.clone()], false);
        insta::with_settings!({
            info => &blue_plane_red_sphere_with_blend,
            description => "Rendering a blue plane and then a red sphere with blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(blue_plane_red_sphere_with_blend.to_raw_data());
        });

        let red_sphere_blue_plane_with_blend =
            Frame3D::new(vec![red_sphere.clone(), blue_plane.clone()], false);
        insta::with_settings!({
            info => &red_sphere_blue_plane_with_blend,
            description => "Rendering a red sphere and then a blue plane with blend",
            omit_expression => true,
        }, {
            insta::assert_ron_snapshot!(red_sphere_blue_plane_with_blend.to_raw_data());
        });
    }
}

//! This module handles the inidividual objects in frames.

use super::RGBArray;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};

mod basic_shapes;
mod planes;
mod splines;

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
                self.render_plane_into_slice(normal, k, threshold, data);
            }

            Object::Sphere { center, radius } => {
                self.render_sphere_into_slice(center, radius, data)
            }

            Object::SplitPlane {
                normal,
                k,
                blend,
                positive_side_colour,
                negative_side_colour,
            } => {
                self.render_split_plane_into_slice(
                    normal,
                    k,
                    blend,
                    positive_side_colour,
                    negative_side_colour,
                    data,
                );
            }

            Object::CatmullRomSpline {
                ref points,
                threshold,
                start_colour,
                end_colour,
            } => {
                self.render_catmull_rom_spline_into_slice(
                    points,
                    threshold,
                    start_colour,
                    end_colour,
                    data,
                );
            }
        }
    }

    /// Work out the amount to fade (in [0, 1]) given a distance into the fade zone.
    fn get_fade(&self, distance: f32) -> f32 {
        (1. - distance / self.fadeoff).clamp(0., 1.)
    }

    /// Set the light colour according to the appropriate fade off, given a distance into the fade
    /// zone and a mut reference to the colour to be changed.
    fn set_light_colour_by_fade(
        &self,
        colour: RGBArray,
        distance: f32,
        light_colour: &mut RGBArray,
    ) {
        let fade = self.get_fade(distance);
        let [r, g, b] = colour;
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

    /// A plane where every point one side gets one colour, and every point on the other side gets
    /// a different colour.
    ///
    /// When used as part of a [`FrameObject`], the [`colour`](FrameObject::colour) field of the
    /// `FrameObject` is ignored and the colours of this variant are used instead.
    SplitPlane {
        /// The normal vector of the plane.
        normal: Vec3,

        /// The result of dotting the direction vector with any point on the plane.
        k: f32,

        /// The distance from the plane where the
        /// [`positive_side_colour`](Object::SplitPlane::positive_side_colour) and
        /// [`negative_side_colour`](Object::SplitPlane::negative_side_colour) are applied in full.
        ///
        /// If a point is less than this distance from the plane, its colour will be linearly
        /// interpolated betweed the two side colours.
        ///
        /// Measured in GIFT coordinates. Should never be negative.
        blend: f32,

        /// The colour given to points on the positive side of the plane.
        positive_side_colour: RGBArray,

        /// The colour given to points on the negative side of the plane.
        negative_side_colour: RGBArray,
    },

    /// A sphere with a center and radius.
    Sphere {
        /// The coordinates of the center of the sphere.
        center: Vec3,

        /// The radius of the sphere.
        radius: f32,
    },

    /// A [centripetal Catmull-Rom spline](https://en.wikipedia.org/wiki/Centripetal_Catmull%E2%80%93Rom_spline)
    /// interpolating a sequence of points with a colour gradient.
    ///
    /// When used as part of a [`FrameObject`], the [`colour`](FrameObject::colour) field of the
    /// `FrameObject` is ignored and the colours of this variant are used instead.
    CatmullRomSpline {
        /// The points to interpolate.
        points: Box<[Vec3]>,

        /// The maximum distance from this object where lights will be counted as part of the
        /// object.
        threshold: f32,

        /// The colour at the start of the spline.
        start_colour: RGBArray,

        /// The colour at the end of the spline.
        end_colour: RGBArray,
    },
}

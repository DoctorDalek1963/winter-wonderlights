//! This module handles the inidividual objects in frames.

use super::RGBArray;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace, warn};
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
                        self.set_light_colour_by_fade(self.colour, dist - threshold, light_colour);
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

                        // 3D Pythagoras
                        f32::sqrt(dx.mul_add(dx, dy.mul_add(dy, dz * dz)))
                    };
                    trace!(?point, ?dist, "Distance from point to center");

                    if dist <= radius {
                        *light_colour = self.colour;
                    } else if dist > radius && dist <= radius + self.fadeoff {
                        self.set_light_colour_by_fade(self.colour, dist - radius, light_colour);
                    }
                }
            }

            Object::SplitPlane {
                normal,
                k,
                blend,
                positive_side_colour,
                negative_side_colour,
            } => {
                for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                    // Get the signed distance from this point to the plane
                    let signed_dist = (normal.dot(point.into()) - k) / normal.length();
                    trace!(?point, ?signed_dist, "Distance from point to plane");

                    // If distance is less than the threshold, then it's part of the plane
                    if signed_dist >= blend {
                        *light_colour = positive_side_colour;
                    } else if signed_dist <= -blend {
                        *light_colour = negative_side_colour;
                    } else {
                        // Interpolate between colours

                        // In range [0, 1]
                        let dist_from_pos_boundary = (blend - signed_dist) / (2. * blend);

                        // colour = pos * (1-t) + neg * t
                        *light_colour = {
                            let pos_col: [f32; 3] = [
                                positive_side_colour[0] as f32 * (1. - dist_from_pos_boundary),
                                positive_side_colour[1] as f32 * (1. - dist_from_pos_boundary),
                                positive_side_colour[2] as f32 * (1. - dist_from_pos_boundary),
                            ];
                            let neg_col: [f32; 3] = [
                                negative_side_colour[0] as f32 * dist_from_pos_boundary,
                                negative_side_colour[1] as f32 * dist_from_pos_boundary,
                                negative_side_colour[2] as f32 * dist_from_pos_boundary,
                            ];

                            [
                                (pos_col[0] + neg_col[0]) as u8,
                                (pos_col[1] + neg_col[1]) as u8,
                                (pos_col[2] + neg_col[2]) as u8,
                            ]
                        };
                    }
                }
            }

            Object::CatmullRomSpline {
                ref points,
                threshold,
                start_colour,
                end_colour,
            } => {
                if let ((Some(&first), Some(&second)), (Some(&last), Some(&penultimate))) = (
                    (points.first(), points.get(1)),
                    (points.last(), points.get(points.len().saturating_sub(2))),
                ) {
                    let control_points = {
                        let mut v = Vec::with_capacity(points.len() + 2);
                        v.push(first + (first - second));
                        v.extend(points.iter());
                        v.push(last + (last - penultimate));
                        v
                    };

                    /// The number of steps between each control point.
                    const STEPS: usize = 20;

                    let mut interpolated_points =
                        Vec::with_capacity((control_points.len() - 3) * STEPS);
                    for &[p0, p1, p2, p3] in control_points.array_windows() {
                        for t in (0..STEPS).map(|x| x as f32 / STEPS as f32) {
                            interpolated_points
                                .push(interpolate_catmull_rom_segment(p0, p1, p2, p3, t));
                        }
                    }

                    for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                        let dist = interpolated_points
                            .iter()
                            .map(|&interpolated_point| {
                                interpolated_point.distance(Vec3::from(point))
                            })
                            .fold(
                                f32::INFINITY,
                                |acc, dist| if dist < acc { dist } else { acc },
                            );

                        if dist <= threshold {
                            // TODO: Work out colour gradient
                            *light_colour = start_colour;
                        } else if dist <= threshold + self.fadeoff {
                            self.set_light_colour_by_fade(start_colour, dist, light_colour);
                        }
                    }
                } else {
                    warn!(
                        "TODO: Render Catmull-Rom spline with 1 or 0 points. Delegate to sphere?"
                    );
                };
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

/// Interpolate a segment of a centripetal Catmull-Rom spline with the 4 given points and the `t`
/// value.
fn interpolate_catmull_rom_segment(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    /// The "tightness" of the curve.
    const ALPHA: f32 = 0.5;

    debug_assert!(t >= 0.0 && t <= 1.0, "t must be in [0, 1]");

    let t0 = 0.0;
    let t1 = ((p1 - p0).length()).powf(ALPHA) + t0;
    let t2 = ((p2 - p1).length()).powf(ALPHA) + t1;
    let t3 = ((p3 - p2).length()).powf(ALPHA) + t2;

    let t = t1 + (t2 - t1) * t;

    let a1 = p0 * ((t1 - t) / (t1 - t0)) + p1 * ((t - t0) / (t1 - t0));
    let a2 = p1 * ((t2 - t) / (t2 - t1)) + p2 * ((t - t1) / (t2 - t1));
    let a3 = p2 * ((t3 - t) / (t3 - t2)) + p3 * ((t - t2) / (t3 - t2));

    let b1 = a1 * ((t2 - t) / (t2 - t0)) + a2 * ((t - t0) / (t2 - t0));
    let b2 = a2 * ((t3 - t) / (t3 - t1)) + a3 * ((t - t1) / (t3 - t1));

    b1 * ((t2 - t) / (t2 - t1)) + b2 * ((t - t1) / (t2 - t1))
}

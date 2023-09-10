//! This module provides method for drawing splines.

use crate::{FrameObject, RGBArray};
use glam::Vec3;
use ww_gift_coords::COORDS;

#[cfg(doc)]
use crate::Object;

impl FrameObject {
    pub(super) fn render_catmull_rom_spline_into_slice(
        &self,
        points: &[Vec3],
        threshold: f32,
        start_colour: RGBArray,
        end_colour: RGBArray,
        data: &mut [RGBArray],
    ) {
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

            let mut interpolated_points = Vec::with_capacity((control_points.len() - 3) * STEPS);
            for &[p0, p1, p2, p3] in control_points.array_windows() {
                for t in (0..STEPS).map(|x| x as f32 / STEPS as f32) {
                    interpolated_points.push(interpolate_catmull_rom_segment(p0, p1, p2, p3, t));
                }
            }

            for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                let dist = interpolated_points
                    .iter()
                    .map(|&interpolated_point| interpolated_point.distance(Vec3::from(point)))
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
            debug_assert!(points.len() <= 1, "We should only have 1 or 0 points here");

            if let Some(&point) = points.first() {
                self.render_sphere_into_slice(point, threshold, data);
            }
        };
    }
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

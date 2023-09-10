//! This module provides method for drawing splines.

use crate::{FrameObject, RGBArray};
use glam::Vec3;
use ww_gift_coords::COORDS;

#[cfg(doc)]
use crate::Object;

impl FrameObject {
    /// Render a centripetal Catmull-Rom spline into the slice. See [`Object::CatmullRomSpline`].
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
            /// The number of steps between each control point.
            const STEPS: usize = 20;

            let control_points = {
                let mut v = Vec::with_capacity(points.len() + 2);
                v.push(first + (first - second));
                v.extend(points.iter());
                v.push(last + (last - penultimate));
                v
            };

            let mut interpolated_points = Vec::with_capacity((control_points.len() - 3) * STEPS);
            for &[p0, p1, p2, p3] in control_points.array_windows() {
                for t in (0..STEPS).map(|x| x as f32 / STEPS as f32) {
                    interpolated_points.push(interpolate_catmull_rom_segment(p0, p1, p2, p3, t));
                }
            }

            let interp_len = interpolated_points.len() as f32;

            for (light_colour, &point) in data.iter_mut().zip(COORDS.coords()) {
                let (idx, dist) = interpolated_points
                    .iter()
                    .map(|&interpolated_point| interpolated_point.distance(Vec3::from(point)))
                    .enumerate()
                    .fold((0, f32::INFINITY), |(acc_idx, acc_dist), (idx, dist)| {
                        if dist < acc_dist {
                            (idx, dist)
                        } else {
                            (acc_idx, acc_dist)
                        }
                    });

                if dist <= threshold + self.fadeoff {
                    let proportion = idx as f32 / interp_len;
                    let [sr, sg, sb] = start_colour;
                    let [er, eg, eb] = end_colour;
                    let colour = [
                        (1. - proportion)
                            .mul_add(sr as f32, proportion * er as f32)
                            .clamp(0., 255.)
                            .floor() as u8,
                        (1. - proportion)
                            .mul_add(sg as f32, proportion * eg as f32)
                            .clamp(0., 255.)
                            .floor() as u8,
                        (1. - proportion)
                            .mul_add(sb as f32, proportion * eb as f32)
                            .clamp(0., 255.)
                            .floor() as u8,
                    ];

                    if dist <= threshold {
                        *light_colour = colour;
                    } else {
                        self.set_light_colour_by_fade(colour, dist, light_colour);
                    }
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
    debug_assert!((0.0..=1.0).contains(&t), "t must be in [0, 1]");

    let t0 = 0.0;
    let t1 = ((p1 - p0).length()).sqrt() + t0;
    let t2 = ((p2 - p1).length()).sqrt() + t1;
    let t3 = ((p3 - p2).length()).sqrt() + t2;

    let t = (t2 - t1).mul_add(t, t1);

    let a1 = p0 * ((t1 - t) / (t1 - t0)) + p1 * ((t - t0) / (t1 - t0));
    let a2 = p1 * ((t2 - t) / (t2 - t1)) + p2 * ((t - t1) / (t2 - t1));
    let a3 = p2 * ((t3 - t) / (t3 - t2)) + p3 * ((t - t2) / (t3 - t2));

    let b1 = a1 * ((t2 - t) / (t2 - t0)) + a2 * ((t - t0) / (t2 - t0));
    let b2 = a2 * ((t3 - t) / (t3 - t1)) + a3 * ((t - t1) / (t3 - t1));

    b1 * ((t2 - t) / (t2 - t1)) + b2 * ((t - t1) / (t2 - t1))
}

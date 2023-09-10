//! This module provides method for drawing planes.

use crate::{FrameObject, RGBArray};
use glam::Vec3;
use tracing::trace;
use ww_gift_coords::COORDS;

#[cfg(doc)]
use crate::Object;

impl FrameObject {
    /// Render a plane into the slice. See [`Object::Plane`].
    pub(super) fn render_plane_into_slice(
        &self,
        normal: Vec3,
        k: f32,
        threshold: f32,
        data: &mut [RGBArray],
    ) {
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

    /// Render a split plane into the slice. See [`Object::SplitPlane`].
    pub(super) fn render_split_plane_into_slice(
        &self,
        normal: Vec3,
        k: f32,
        blend: f32,
        positive_side_colour: RGBArray,
        negative_side_colour: RGBArray,
        data: &mut [RGBArray],
    ) {
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
}

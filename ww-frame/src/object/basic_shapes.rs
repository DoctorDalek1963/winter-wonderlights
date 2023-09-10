//! This module provides methods for drawing basic shapes like spheres.

use crate::{FrameObject, RGBArray};
use glam::Vec3;
use tracing::trace;
use ww_gift_coords::COORDS;

#[cfg(doc)]
use crate::Object;

impl FrameObject {
    /// Render a sphere into the slice. See [`Object::Sphere`].
    pub(super) fn render_sphere_into_slice(
        &self,
        center: Vec3,
        radius: f32,
        data: &mut [RGBArray],
    ) {
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
}

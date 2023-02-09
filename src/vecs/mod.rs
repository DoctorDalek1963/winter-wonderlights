//! This module handles everything to do with 3D vectors and points (position vectors).

mod ops;
mod traits;

/// A 3D vector with f64 values.
///
/// This type can also represent a point, interpreted as a position vector. The vertical axis is z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    /// The x component.
    x: f64,
    /// The y component.
    y: f64,
    /// The z component.
    z: f64,
}

impl Vec3 {
    /// Create a new vector with the given values.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Create a new vector with 3 lots of the given value.
    pub fn splat(n: f64) -> Self {
        Self { x: n, y: n, z: n }
    }

    /// Return the length of the vector.
    #[inline]
    pub fn length(&self) -> f64 {
        let Self { x, y, z } = *self;
        f64::sqrt(x * x + y * y + z * z)
    }

    /// The dot product of two vectors.
    #[rustfmt::skip]
    pub fn dot(&self, other: &Self) -> f64 {
        let Self { x: x1, y: y1, z: z1 } = *self;
        let Self { x: x2, y: y2, z: z2 } = *other;
        x1 * x2 + y1 * y2 + z1 * z2
    }

    /// Normalise the vector to have length 1.
    #[must_use = "This function returns a normalised vector and does not mutate in-place"]
    pub fn normalise(&self) -> Self {
        let l = 1. / self.length();
        *self * l
    }
}

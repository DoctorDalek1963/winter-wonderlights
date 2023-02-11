//! This module handles everything to do with 3D vectors and points (position vectors).

use serde::{Deserialize, Serialize};

mod ops;
mod traits;

/// A 3D vector with f64 values.
///
/// This type can also represent a point, interpreted as a position vector. The vertical axis is z.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    #[must_use]
    pub fn length(&self) -> f64 {
        let Self { x, y, z } = *self;
        f64::sqrt(x * x + y * y + z * z)
    }

    /// The dot product of two vectors.
    #[rustfmt::skip]
    #[must_use]
    pub fn dot(&self, other: &Self) -> f64 {
        let Self { x: x1, y: y1, z: z1 } = *self;
        let Self { x: x2, y: y2, z: z2 } = *other;
        x1 * x2 + y1 * y2 + z1 * z2
    }

    /// Compute the cross product of this vector with the other one.
    #[rustfmt::skip]
    #[must_use]
    pub fn cross(&self, other: &Self) -> Self {
        let Self { x: a1, y: a2, z: a3 } = *self;
        let Self { x: b1, y: b2, z: b3 } = *other;
        Self {
            x: a2 * b3 - a3 * b2,
            y: a3 * b1 - a1 * b3,
            z: a1 * b2 - a2 * b1,
        }
    }

    /// Normalise the vector to have length 1.
    #[must_use = ".normalise() returns a normalised vector and does not mutate in-place"]
    pub fn normalise(&self) -> Self {
        let l = 1. / self.length();
        *self * l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;

    #[test]
    fn length_and_normalise_test() {
        let v1 = Vec3::new(1., 2., 3.);
        let v2 = Vec3::splat(1.5);
        let v3 = Vec3::new(3., -2.1, 0.);
        let v4 = Vec3::new(-1.13, 9., 0.2);
        let v5 = Vec3::new(2.43, -0.4, 1.);
        let v6 = Vec3::new(2.3, 1.2, -19.);

        // All calculated values courtesy of GNU Octave
        assert!(approx_eq!(f64, v1.length(), 3.741657386773941));
        assert!(approx_eq!(f64, v2.length(), 2.598076211353316));
        assert!(approx_eq!(f64, v3.length(), 3.661966684720111));
        assert!(approx_eq!(f64, v4.length(), 9.072866140310900));
        assert!(approx_eq!(f64, v5.length(), 2.657987960845572));
        assert!(approx_eq!(f64, v6.length(), 19.17628744048232));

        assert!(approx_eq!(
            Vec3,
            dbg!(v1.normalise()),
            Vec3::new(0.267261241912424, 0.534522483824849, 0.801783725737273),
            epsilon = 0.00000000001
        ));
        assert!(approx_eq!(
            Vec3,
            v2.normalise(),
            Vec3::splat(0.577350269189626),
            epsilon = 0.00000000001
        ));
        assert!(approx_eq!(
            Vec3,
            v3.normalise(),
            Vec3::new(0.819231920519040, -0.573462344363328, 0.),
            epsilon = 0.00000000001
        ));
        assert!(approx_eq!(
            Vec3,
            v4.normalise(),
            Vec3::new(
                -1.245471918713085e-01,
                9.919687848157316e-01,
                2.204375077368292e-02
            ),
            epsilon = 0.00000000001
        ));
        assert!(approx_eq!(
            Vec3,
            v5.normalise(),
            Vec3::new(0.914225359857144, -0.150489771169900, 0.376224427924751),
            epsilon = 0.00000000001
        ));
        assert!(approx_eq!(
            Vec3,
            v6.normalise(),
            Vec3::new(
                1.199397958097227e-01,
                6.257728477029012e-02,
                -9.908070088629269e-01
            ),
            epsilon = 0.00000000001
        ));
    }

    #[test]
    fn products_test() {
        assert!(approx_eq!(
            f64,
            Vec3::new(1., 2., 3.).dot(&Vec3::splat(1.5)),
            9.
        ));
        assert!(approx_eq!(
            f64,
            Vec3::new(3., -2.1, 0.).dot(&Vec3::new(-1.13, 9., 0.2)),
            -22.29
        ));
        assert!(approx_eq!(
            f64,
            Vec3::new(2.43, -0.4, 1.).dot(&Vec3::new(2.3, 1.2, -19.)),
            -13.891
        ));

        assert!(approx_eq!(
            Vec3,
            Vec3::new(1., 2., 3.).cross(&Vec3::splat(1.5)),
            Vec3::new(-1.5, 3., -1.5)
        ));
        assert!(approx_eq!(
            Vec3,
            Vec3::new(3., -2.1, 0.).cross(&Vec3::new(-1.13, 9., 0.2)),
            Vec3::new(-0.42, -0.6, 24.627)
        ));
        assert!(approx_eq!(
            Vec3,
            Vec3::new(2.43, -0.4, 1.).cross(&Vec3::new(2.3, 1.2, -19.)),
            Vec3::new(6.4, 48.47, 3.836)
        ));
    }
}

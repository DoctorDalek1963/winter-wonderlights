//! This module implements traits on [`Vec3`].

use super::Vec3;
use crate::PointF;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

impl Distribution<Vec3> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        Vec3 {
            x: rng.gen_range(-1.0..=1.0),
            y: rng.gen_range(-1.0..=1.0),
            z: rng.gen_range(-1.0..=1.0),
        }
        .normalise()
    }
}

impl From<PointF> for Vec3 {
    fn from((x, y, z): PointF) -> Self {
        Self { x, y, z }
    }
}

impl From<Vec3> for PointF {
    fn from(Vec3 { x, y, z }: Vec3) -> Self {
        (x, y, z)
    }
}

#[cfg(test)]
impl float_cmp::ApproxEq for Vec3 {
    type Margin = float_cmp::F64Margin;

    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        let margin: Self::Margin = margin.into();

        self.x.approx_eq(other.x, margin)
            && self.y.approx_eq(other.y, margin)
            && self.z.approx_eq(other.z, margin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;

    #[test]
    fn from_into_test() {
        let p: PointF = (1., 2., 3.);
        let v = Vec3 {
            x: 1.,
            y: 2.,
            z: 3.,
        };

        assert_eq!(Into::<Vec3>::into(p), v, "PointF.into() == Vec3");
        assert_eq!(Vec3::from(p), v, "Vec3::from(PointF) == Vec3");

        assert_eq!(Into::<PointF>::into(v), p, "Vec3.into() == PointF");
        assert_eq!(PointF::from(v), p, "PointF::from(Vec3) == PointF");
    }

    #[test]
    fn approx_eq_test() {
        assert!(approx_eq!(
            Vec3,
            Vec3::new(1., 2., 3.),
            Vec3::new(
                1.000000000000000001,
                2.000000000000000001,
                2.999999999999999999
            )
        ));
    }
}

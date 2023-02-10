//! This module implements std operations on [`Vec3`].

use super::Vec3;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        *self = *self * rhs;
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        *self = *self / rhs;
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_test() {
        assert_eq!(
            Vec3::new(1.2, 3.6, -2.1) + Vec3::new(0.5, -1.1, 0.85),
            Vec3::new(1.7, 2.5, -1.25),
            "Vec3 as Add"
        );
        assert_eq!(
            Vec3::new(1.2, 3.6, -2.1) - Vec3::new(0.5, -1.1, 0.85),
            Vec3::new(0.7, 4.7, -2.95),
            "Vec3 as Sub"
        );
        assert_eq!(
            Vec3::new(1.2, 3.6, -2.1) * 2.,
            Vec3::new(2.4, 7.2, -4.2),
            "Vec3 as Mul (Vec on left)"
        );
        assert_eq!(
            2. * Vec3::new(1.2, 3.6, -2.1),
            Vec3::new(2.4, 7.2, -4.2),
            "Vec3 as Mul (Vec on right)"
        );
        assert_eq!(
            Vec3::new(1.2, 3.6, -2.1) / 2.,
            Vec3::new(0.6, 1.8, -1.05),
            "Vec3 as Div"
        );
        assert_eq!(
            -Vec3::new(1.2, 3.6, -2.1),
            Vec3::new(-1.2, -3.6, 2.1),
            "Vec3 as Neg"
        );
    }
}

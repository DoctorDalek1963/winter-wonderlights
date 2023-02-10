//! This module handles everything to do with GIFT coordinates.
//!
//! The GIFT (short for Geographic Information For Trees) coordinate system is one in which each
//! light on the tree is given a 3D coordinate with `f64` components. These coordinates are
//! normalised in the following way:
//!
//! The base of the tree is assumed to be a circle, so the x and y
//! values are normalised so that everything is between -1 and 1, but the scale doesn't change.
//! Then the z components are shifted so they're all positive, and scaled to the same scale as the
//! x and y values. This means th minimum z values is 0, and the maximum z value depends on the
//! other coordinates.

use crate::PointF;
use color_eyre::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;

lazy_static! {
    /// The GIFTCoords loaded from `coords.gift`.
    pub static ref COORDS: GIFTCoords =
        GIFTCoords::from_file("coords.gift").expect("We need the coordinates to build the tree");
}

/// A simple struct to hold and manage GIFT coordinates. See the module documentation for details.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GIFTCoords {
    /// The coordinates of the lights themselves.
    coords: Vec<(f64, f64, f64)>,

    /// The maximum z value, used for caching.
    ///
    /// See [`GIFTCoords::max_z()`].
    max_z: f64,

    /// The total number of lights.
    lights_num: usize,
}

impl GIFTCoords {
    /// Create a set of GIFT coordinates by normalising a list of integer coordinates.
    pub fn from_int_coords(int_coords: &Vec<(i32, i32, i32)>) -> Option<Self> {
        let xs = int_coords.iter().map(|&(x, _, _)| x);
        let ys = int_coords.iter().map(|&(_, y, _)| y);
        let zs = int_coords.iter().map(|&(_, _, z)| z);

        let min_z = zs.clone().min()?;

        let mid_x = f64::from(xs.clone().min()? + xs.clone().max()?) / 2.;
        let mid_y = f64::from(ys.clone().min()? + ys.clone().max()?) / 2.;

        // Centered on 0
        let centered_xs = xs.into_iter().map(|x| f64::from(x) - mid_x);
        let centered_ys = ys.into_iter().map(|y| f64::from(y) - mid_y);

        let max_x_y: f64 = [
            centered_xs.clone().reduce(f64::max)?.abs(),
            centered_xs.clone().reduce(f64::min)?.abs(),
            centered_ys.clone().reduce(f64::max)?.abs(),
            centered_ys.clone().reduce(f64::min)?.abs(),
        ]
        .into_iter()
        .reduce(f64::max)?;

        let new_xs = centered_xs.into_iter().map(|x| x / max_x_y);
        let new_ys = centered_ys.into_iter().map(|y| y / max_x_y);
        let new_zs = zs.into_iter().map(|z| f64::from(z - min_z) / max_x_y);

        let max_z = new_zs.clone().reduce(f64::max)?;
        let coords: Vec<(f64, f64, f64)> = new_xs
            .zip(new_ys)
            .zip(new_zs)
            .map(|((x, y), z)| (x, y, z))
            .collect();
        let lights_num = coords.len();

        Some(Self {
            coords,
            max_z,
            lights_num,
        })
    }

    /// Read the coordinate list from a file.
    ///
    /// The file should be a `Vec<(f64, f64, f64)>` encoded with `bincode`.
    pub fn from_file(filename: &str) -> Result<Self> {
        let coords: Vec<(f64, f64, f64)> = bincode::deserialize(&fs::read(filename)?)?;
        let max_z = coords.iter().fold(0.0, |acc, &(_, _, z)| f64::max(acc, z));
        let lights_num = coords.len();

        Ok(Self {
            coords,
            max_z,
            lights_num,
        })
    }

    /// Check if the given point is within the bounding box of the coordinates.
    pub fn is_within_bounding_box(&self, (x, y, z): PointF) -> bool {
        (-1.0..=1.0).contains(&x) && (-1.0..=1.0).contains(&y) && (0.0..=self.max_z).contains(&z)
    }

    /// Calculate the distance between the point and the bounding box. Return 0 if the point is
    /// inside.
    pub fn distance_from_bounding_box(&self, (x, y, z): PointF) -> f64 {
        if self.is_within_bounding_box((x, y, z)) {
            0.
        } else {
            let dx = (-1. - x).max(0.).max(x - 1.);
            let dy = (-1. - y).max(0.).max(y - 1.);
            let dz = (-z).max(0.).max(z - self.max_z);

            f64::sqrt(dx * dx + dy * dy + dz * dz)
        }
    }

    /// Return the central point of the bounding box.
    pub fn center(&self) -> PointF {
        (0.5, 0.5, self.max_z / 2.)
    }

    /// The vec of coordinates themselves.
    pub fn coords(&self) -> &Vec<PointF> {
        &self.coords
    }

    /// The maximum z value.
    ///
    /// The minimum z value is 0, and the minimum and maximum x and y values and -1 and 1.
    pub fn max_z(&self) -> f64 {
        self.max_z
    }

    /// The total number of lights.
    pub fn lights_num(&self) -> usize {
        self.lights_num
    }
}

#[cfg(test)]
impl float_cmp::ApproxEq for GIFTCoords {
    type Margin = float_cmp::F64Margin;

    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        let margin: Self::Margin = margin.into();
        self.max_z.approx_eq(other.max_z, margin)
            && self
                .coords
                .into_iter()
                .zip(other.coords)
                .all(|((x1, y1, z1), (x2, y2, z2))| {
                    x1.approx_eq(x2, margin) && y1.approx_eq(y2, margin) && z1.approx_eq(z2, margin)
                })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;

    const MAX_Z: f64 = 3.592079207920792;

    #[inline]
    fn gift_coords() -> GIFTCoords {
        GIFTCoords {
            coords: include!("float_coords.txt"),
            max_z: MAX_Z,
            lights_num: 500,
        }
    }

    #[test]
    fn from_int_coords_test() {
        let int_coords: Vec<(i32, i32, i32)> = include!("int_coords.txt");

        assert!(
            approx_eq!(
                GIFTCoords,
                GIFTCoords::from_int_coords(&int_coords).unwrap(),
                gift_coords()
            ),
            "Testing no translation"
        );
        assert!(
            approx_eq!(
                GIFTCoords,
                GIFTCoords::from_int_coords(
                    &int_coords
                        .iter()
                        .map(|&(x, y, z)| (x + 500, y, z))
                        .collect()
                )
                .unwrap(),
                gift_coords()
            ),
            "Testing simple translation in x"
        );
        assert!(
            approx_eq!(
                GIFTCoords,
                GIFTCoords::from_int_coords(
                    &int_coords
                        .iter()
                        .map(|&(x, y, z)| (x - 493, y + 112, z + 1000))
                        .collect()
                )
                .unwrap(),
                gift_coords()
            ),
            "Testing multiple translations"
        );
    }

    #[test]
    fn from_file_test() {
        assert_eq!(GIFTCoords::from_file("coords.gift").unwrap(), gift_coords());
    }

    #[test]
    fn within_bounding_box_test() {
        for p in [
            (0., 0., 0.),
            (0.5, 0.5, 1.),
            (0.5, -1., 1.83),
            (-0.59, 0.22, 2.454),
        ] {
            assert!(COORDS.is_within_bounding_box(p));
        }

        for p in [(1.01, 0., 0.5), (0., 0., 5.), (-1.4, 2.63, 5.)] {
            assert!(!COORDS.is_within_bounding_box(p));
        }
    }

    #[test]
    fn distance_from_bounding_box_test() {
        for p in [
            (0., 0., 0.),
            (0.5, 0.5, 1.),
            (0.5, -1., 1.83),
            (-0.59, 0.22, 2.454),
        ] {
            assert!(approx_eq!(f64, COORDS.distance_from_bounding_box(p), 0.));
        }

        assert!(approx_eq!(
            f64,
            COORDS.distance_from_bounding_box((1.5, 0., 0.)),
            0.5
        ));
        assert!(approx_eq!(
            f64,
            COORDS.distance_from_bounding_box((0., 0., 5.)),
            5. - MAX_Z
        ));
        assert!(approx_eq!(
            f64,
            COORDS.distance_from_bounding_box((0., 1.5, 4.)),
            0.6452901460665026 // Hypotenuse
        ));
        dbg!(COORDS.distance_from_bounding_box((-2.1, 1.5, 4.)));
        assert!(approx_eq!(
            f64,
            COORDS.distance_from_bounding_box((-2.1, 1.5, 4.)),
            1.2753036393779047 // Hypotenuse
        ));
    }
}

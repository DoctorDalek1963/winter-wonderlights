//! This crate handles everything to do with GIFT coordinates.
//!
//! The GIFT (short for Geographic Information For Trees) coordinate system is one in which each
//! light on the tree is given a 3D coordinate with `f64` components. These coordinates are
//! normalized in the following way:
//!
//! The base of the tree is assumed to be a circle, so the x and y
//! values are normalized so that everything is between -1 and 1, but the scale doesn't change.
//! Then the z components are shifted so they're all positive, and scaled to the same scale as the
//! x and y values. This means th minimum z values is 0, and the maximum z value depends on the
//! other coordinates.

use color_eyre::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use tracing_unwrap::ResultExt;

/// A point in 3D space with f32 values.
pub type PointF = (f32, f32, f32);

/// Get the name of the file containing the coordinates.
fn get_coords_file_name() -> String {
    format!(
        "{}/coords/{}",
        std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined"),
        std::env::var("COORDS_FILENAME").expect_or_log("COORDS_FILENAME must be defined")
    )
}

lazy_static! {
    /// The GIFTCoords loaded from the file in `COORDS_FILENAME`.
    pub static ref COORDS: GIFTCoords = GIFTCoords::from_file(&get_coords_file_name()).expect_or_log("Failed to load coordinates from file");
}

/// A simple struct to hold and manage GIFT coordinates. See the module documentation for details.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GIFTCoords {
    /// The coordinates of the lights themselves.
    coords: Vec<PointF>,

    /// The maximum z value, used for caching.
    ///
    /// See [`GIFTCoords::max_z()`].
    max_z: f32,

    /// The total number of lights.
    lights_num: usize,
}

impl GIFTCoords {
    /// Create a set of GIFT coordinates by normalising a list of integer coordinates.
    pub fn from_int_coords(int_coords: &[(i32, i32, i32)]) -> Option<Self> {
        Self::from_unnormalized_coords(
            &int_coords
                .iter()
                .map(|&(x, y, z)| (x as f32, y as f32, z as f32))
                .collect::<Vec<_>>(),
        )
    }

    /// Create a set of GIFT coordinates by normalising a list of unnormalised [`f32`] coordinates.
    pub fn from_unnormalized_coords(coords: &[(f32, f32, f32)]) -> Option<Self> {
        let xs = coords.iter().map(|&(x, _, _)| x);
        let ys = coords.iter().map(|&(_, y, _)| y);
        let zs = coords.iter().map(|&(_, _, z)| z);

        let min_z = zs.clone().reduce(f32::min)?;

        let mid_x = (xs.clone().reduce(f32::min)? + xs.clone().reduce(f32::max)?) / 2.;
        let mid_y = (ys.clone().reduce(f32::min)? + ys.clone().reduce(f32::max)?) / 2.;

        // Centered on 0
        let centered_xs = xs.into_iter().map(|x| x - mid_x);
        let centered_ys = ys.into_iter().map(|y| y - mid_y);

        let max_x_y: f32 = [
            centered_xs.clone().reduce(f32::max)?.abs(),
            centered_xs.clone().reduce(f32::min)?.abs(),
            centered_ys.clone().reduce(f32::max)?.abs(),
            centered_ys.clone().reduce(f32::min)?.abs(),
        ]
        .into_iter()
        .reduce(f32::max)?;

        let new_xs = centered_xs.into_iter().map(|x| x / max_x_y);
        let new_ys = centered_ys.into_iter().map(|y| y / max_x_y);
        let new_zs = zs.into_iter().map(|z| (z - min_z) / max_x_y);

        let max_z = new_zs.clone().reduce(f32::max)?;
        let coords: Vec<(f32, f32, f32)> = new_xs
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
    /// The file should be a `Vec<(f32, f32, f32)>` encoded with `bincode`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file can't be read or if it can't be
    /// deserialized.
    pub fn from_file(filename: &str) -> Result<Self> {
        let coords: Vec<(f32, f32, f32)> = bincode::deserialize(&fs::read(filename)?)?;
        let max_z = coords.iter().fold(0.0, |acc, &(_, _, z)| f32::max(acc, z));
        let lights_num = coords.len();

        Ok(Self {
            coords,
            max_z,
            lights_num,
        })
    }

    /// Save the coordinate list to the given file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data can't be serialized or if the file can't be
    /// written to.
    pub fn save_to_file(&self, filename: &str) -> Result<()> {
        let data = bincode::serialize(&self.coords)?;
        fs::write(filename, data)?;
        Ok(())
    }

    /// Check if the given point is within the bounding box of the coordinates.
    pub fn is_within_bounding_box(&self, (x, y, z): PointF) -> bool {
        (-1.0..=1.0).contains(&x) && (-1.0..=1.0).contains(&y) && (0.0..=self.max_z).contains(&z)
    }

    /// Calculate the distance between the point and the bounding box. Return 0 if the point is
    /// inside.
    pub fn distance_from_bounding_box(&self, (x, y, z): PointF) -> f32 {
        if self.is_within_bounding_box((x, y, z)) {
            0.
        } else {
            let dx = (-1. - x).max(0.).max(x - 1.);
            let dy = (-1. - y).max(0.).max(y - 1.);
            let dz = (-z).max(0.).max(z - self.max_z);

            // 3D Pythagoras
            f32::sqrt(dx.mul_add(dx, dy.mul_add(dy, dz * dz)))
        }
    }

    /// Return the central point of the bounding box.
    pub fn center(&self) -> PointF {
        (0., 0., self.max_z / 2.)
    }

    /// The vec of coordinates themselves.
    pub fn coords(&self) -> &Vec<PointF> {
        &self.coords
    }

    /// The maximum z value.
    ///
    /// The minimum z value is 0, and the minimum and maximum x and y values and -1 and 1.
    pub fn max_z(&self) -> f32 {
        self.max_z
    }

    /// The total number of lights.
    pub fn lights_num(&self) -> usize {
        self.lights_num
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;
    use tracing_unwrap::OptionExt;

    impl float_cmp::ApproxEq for GIFTCoords {
        type Margin = float_cmp::F32Margin;

        fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
            let margin: Self::Margin = margin.into();
            self.lights_num == other.lights_num
                && self.max_z.approx_eq(other.max_z, margin)
                && self
                    .coords
                    .into_iter()
                    .zip(other.coords)
                    .all(|((x1, y1, z1), (x2, y2, z2))| {
                        x1.approx_eq(x2, margin)
                            && y1.approx_eq(y2, margin)
                            && z1.approx_eq(z2, margin)
                    })
        }
    }

    const MAX_Z: f32 = 3.592079207920792;

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
                GIFTCoords::from_int_coords(&int_coords).unwrap_or_log(),
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
                        .collect::<Vec<_>>()
                )
                .unwrap_or_log(),
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
                        .collect::<Vec<_>>()
                )
                .unwrap_or_log(),
                gift_coords()
            ),
            "Testing multiple translations"
        );
    }

    #[test]
    fn from_file_test() {
        assert_eq!(
            GIFTCoords::from_file(&format!(
                "{}/coords/2020-matt-parker.gift",
                std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
            ))
            .unwrap_or_log(),
            gift_coords()
        );
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
            assert!(approx_eq!(f32, COORDS.distance_from_bounding_box(p), 0.));
        }

        assert!(approx_eq!(
            f32,
            COORDS.distance_from_bounding_box((1.5, 0., 0.)),
            0.5
        ));
        assert!(approx_eq!(
            f32,
            COORDS.distance_from_bounding_box((0., 0., 5.)),
            5. - MAX_Z
        ));
        assert!(approx_eq!(
            f32,
            COORDS.distance_from_bounding_box((0., 1.5, 4.)),
            0.6452901460665026 // Hypotenuse
        ));
        assert!(approx_eq!(
            f32,
            COORDS.distance_from_bounding_box((-2.1, 1.5, 4.)),
            1.2753036393779047 // Hypotenuse
        ));
    }
}

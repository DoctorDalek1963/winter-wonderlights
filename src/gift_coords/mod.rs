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

/// A simple struct to hold and manage GIFT coordinates. See the module documentation for details.
#[derive(Clone, Debug, PartialEq)]
pub struct GIFTCoords {
    /// The coordinates of the lights themselves.
    coords: Vec<(f64, f64, f64)>,

    /// The maximum z value, used for caching.
    ///
    /// See [`GIFTCoords::max_z()`].
    max_z: f64,
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
        let coords = new_xs
            .zip(new_ys)
            .zip(new_zs)
            .map(|((x, y), z)| (x, y, z))
            .collect();

        Some(Self { coords, max_z })
    }

    /// The vec of coordinates themselves.
    pub fn coords(&self) -> &Vec<(f64, f64, f64)> {
        &self.coords
    }

    /// The maximum z value.
    ///
    /// The minimum z value is 0, and the minimum and maximum x and y values and -1 and 1.
    pub fn max_z(&self) -> f64 {
        self.max_z
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

    #[test]
    fn from_int_coords_test() {
        let int_coords: Vec<(i32, i32, i32)> = include!("int_coords.txt");
        let float_coords: Vec<(f64, f64, f64)> = include!("float_coords.txt");
        let max_z = 3.592079207920792;

        assert!(
            approx_eq!(
                GIFTCoords,
                GIFTCoords::from_int_coords(&int_coords).unwrap(),
                GIFTCoords {
                    coords: float_coords.clone(),
                    max_z
                }
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
                GIFTCoords {
                    coords: float_coords.clone(),
                    max_z
                }
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
                GIFTCoords {
                    coords: float_coords.clone(),
                    max_z
                }
            ),
            "Testing multiple translations"
        );
    }
}

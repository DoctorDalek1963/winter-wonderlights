//! This module just contains [`generate_gift_file`].

use std::collections::HashMap;
use tracing::{debug, info};
use tracing_unwrap::{OptionExt, ResultExt};
use ww_gift_coords::{GIFTCoords, PointF};
use ww_scanner_shared::CompassDirection;

/// A point in an image.
type ImgPoint = (u32, u32);

/// Generate a GIFT file from the photo map and write it to `${DATA_DIR}/scanned_coords.gift`.
pub fn generate_gift_file(photo_map: HashMap<CompassDirection, Vec<(ImgPoint, u8)>>) {
    debug!("Generating GIFT file");

    let root_2_over_2 = f32::sqrt(2.) / 2.;

    let gift_point_list: Vec<Vec<(PointF, u8)>> = photo_map
        .into_iter()
        .map(|(direction, points)| {
            let ((bb_tl_x, bb_tl_y), (bb_br_x, bb_br_y)) = get_bounding_box(&points);

            // How many pixels are there in one GIFT unit?
            let pixels_per_gift_unit = (bb_br_x - bb_tl_x) / 2;

            // The x coordinate of the trunk of the tree
            let middle_x = bb_tl_x + pixels_per_gift_unit;

            let gift_points: Vec<(PointF, u8)> = points
                .into_iter()
                .map(|((px, py), brightness)| {
                    // The z coordinate of this LED in GIFT coordinates
                    let z = {
                        let top_gap_pixels = py - bb_tl_y;
                        let z_pixels = (bb_br_y - bb_tl_y) - top_gap_pixels;
                        z_pixels as f32 / pixels_per_gift_unit as f32
                    };

                    // The horizontal_offset of this LED from the trunk of the tree
                    let horizontal_offset =
                        (px as f32 - middle_x as f32) / pixels_per_gift_unit as f32;

                    let (x, y) = match direction {
                        CompassDirection::North => (-horizontal_offset, 0.),
                        CompassDirection::NorthEast => (
                            -horizontal_offset * root_2_over_2,
                            horizontal_offset * root_2_over_2,
                        ),
                        CompassDirection::East => (0., horizontal_offset),
                        CompassDirection::SouthEast => (
                            horizontal_offset * root_2_over_2,
                            horizontal_offset * root_2_over_2,
                        ),
                        CompassDirection::South => (horizontal_offset, 0.),
                        CompassDirection::SouthWest => (
                            horizontal_offset * root_2_over_2,
                            -horizontal_offset * root_2_over_2,
                        ),
                        CompassDirection::West => (0., -horizontal_offset),
                        CompassDirection::NorthWest => (
                            -horizontal_offset * root_2_over_2,
                            -horizontal_offset * root_2_over_2,
                        ),
                    };

                    ((x, y, z), brightness)
                })
                .collect();

            gift_points
        })
        .collect();

    let lights_num = gift_point_list.first().unwrap().len();
    let mut coordinates = Vec::with_capacity(lights_num);

    let directions_num = gift_point_list.len() as f32;

    for idx in 0..lights_num {
        let (x, y, z) = gift_point_list.iter().map(|points| points[idx]).fold(
            (0., 0., 0.),
            |(acc_x, acc_y, acc_z), ((x, y, z), brightness)| {
                let weight = brightness as f32 / 255.;
                (acc_x + x * weight, acc_y + y * weight, acc_z + z * weight)
            },
        );
        coordinates.push((x / directions_num, y / directions_num, z / directions_num));
    }

    let gift_coords = GIFTCoords::from_unnormalized_coords(&coordinates)
        .expect_or_log("We should be able to normalize the scanned coordinates");

    debug!(?gift_coords, "Generated GIFT coords");

    gift_coords
        .save_to_file(concat!(env!("DATA_DIR"), "/scanned_coords.gift"))
        .expect_or_log("We should be able to save the new GIFT coordinates to the file");

    info!("Successfully created GIFT file");
}

/// Find the bounding box of the given set of points. The first point in the returned tuple is the
/// top left, and the second in the bottom right.
fn get_bounding_box(points: &[(ImgPoint, u8)]) -> (ImgPoint, ImgPoint) {
    points.iter().fold(
        ((u32::MAX, u32::MAX), (0, 0)),
        |((tlx, tly), (brx, bry)), &((x, y), _brightness)| {
            ((tlx.min(x), tly.min(y)), (brx.max(x), bry.max(y)))
        },
    )
}

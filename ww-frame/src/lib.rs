//! This crate provides functionality for specifiying and using 3D frames.

#![feature(array_windows)]
#![feature(lint_reasons)]

use glam::Vec3;
use rand::Rng;

mod frame;
mod object;

pub use self::{
    frame::{Frame3D, FrameType},
    object::{FrameObject, Object},
};

/// An RGB colour.
pub type RGBArray = [u8; 3];

/// Generate a random `Vec3` with positive or negative elements, and normalize it.
pub fn random_vector<R: Rng + ?Sized>(rng: &mut R) -> Vec3 {
    (rng.gen::<Vec3>() - Vec3::new(0.5, 0.5, 0.5)).normalize()
}

//! This module provides functionality for specifiying and using 3D frames.

use crate::gift_coords::COORDS;
use tracing::debug;

mod object;

pub use self::object::{FrameObject, Object};

/// An RGB colour.
pub type RGBArray = [u8; 3];

/// A type of frame data.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameType {
    /// A frame indicating that all the lights are off.
    Off,

    /// The colour for each light on the chain.
    ///
    /// The driver is expected to handle the cases where the Vec is too long or too short and
    /// doesn't corespond one-to-one to the lights, but this behaviour is specific to individual
    /// drivers and should not be relied on.
    RawData(Vec<RGBArray>),

    /// A sophisticated 3D frame, made of several objects.
    Frame3D(Frame3D),
}

/// A 3D frame, made of several objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame3D {
    /// The vec of objects in the frame.
    ///
    /// An object later in the vec will override any object previous in the list. No blending of
    /// colours is performed.
    pub objects: Vec<FrameObject>,
}

impl Frame3D {
    /// Convert the frame to a raw data vec, using [`COORDS`] to know where the lights are.
    pub fn to_raw_data(&self) -> Vec<RGBArray> {
        let mut data: Vec<RGBArray> = vec![[0, 0, 0]; COORDS.lights_num()];
        debug!(?data);

        for frame_object in &self.objects {
            frame_object.render_into_vec(&mut data);
        }

        debug!(?data);
        data
    }
}

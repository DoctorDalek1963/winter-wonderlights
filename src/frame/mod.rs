//! This module provides functionality for specifiying and using 3D frames.

use crate::PointF;

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
    objects: Vec<Object>,
}

/// An object in a 3D frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    /// A straight line through 2 points.
    Line(PointF, PointF),

    /// A sphere with a center and radius.
    Sphere(PointF, f64),
}

//! This module contains purely mathematical effects.

pub mod moving_plane;
pub mod split_plane;

#[cfg(feature = "effect-impls")]
pub use self::{moving_plane::MovingPlane, split_plane::SplitPlane};

#[cfg(feature = "config-impls")]
pub use self::{moving_plane::MovingPlaneConfig, split_plane::SplitPlaneConfig};

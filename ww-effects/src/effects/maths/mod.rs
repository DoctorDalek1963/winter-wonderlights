//! This module contains purely mathematical effects.

pub mod moving_plane;

#[cfg(feature = "effect-impls")]
pub use self::moving_plane::MovingPlane;

#[cfg(feature = "config-impls")]
pub use self::moving_plane::MovingPlaneConfig;

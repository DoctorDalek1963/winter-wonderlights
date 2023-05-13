//! This module contains purely mathematical effects.

pub mod lava_lamp;

#[cfg(feature = "effect-impls")]
pub use self::lava_lamp::LavaLamp;

#[cfg(feature = "config-impls")]
pub use self::lava_lamp::LavaLampConfig;

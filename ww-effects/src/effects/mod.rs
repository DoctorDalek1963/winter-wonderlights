//! This module handles all implementations of [`Effect`]s and [`EffectConfig`]s.
//!
//! You probably want to use it like so:
//!
//! ```rust
//! // If you want just the effects:
//! use ww_effects::effects::effects::*;
//!
//! // If you want just the configs:
//! use ww_effects::effects::configs::*;
//!
//! // If you want everything:
//! use ww_effects::effects::*;
//! ```

#[cfg(doc)]
use crate::traits::{Effect, EffectConfig};

/// Asynchronously sleep for the specified duration and await it when running normally.
///
/// The sleep call gets completely removed for test and bench builds.
#[cfg(feature = "effect-impls")]
macro_rules! sleep {
    ( $dur:expr ) => {
        #[cfg(not(any(test, feature = "bench")))]
        ::tokio::time::sleep($dur).await
    };
}

#[cfg(feature = "effect-impls")]
pub(crate) use sleep;

pub mod aesthetic;
pub mod debug;
pub mod maths;

#[cfg(feature = "effect-impls")]
pub use self::effects::*;

/// This module re-exports all the [`Effect`] implementors.
#[cfg(feature = "effect-impls")]
pub mod effects {
    pub use super::{
        aesthetic::LavaLamp,
        debug::{DebugBinaryIndex, DebugOneByOne},
        maths::MovingPlane,
    };
}

#[cfg(feature = "config-impls")]
pub use self::configs::*;

/// This module re-exports all the [`EffectConfig`] implementors.
#[cfg(feature = "config-impls")]
pub mod configs {
    pub use super::{
        aesthetic::LavaLampConfig,
        debug::{DebugBinaryIndexConfig, DebugOneByOneConfig},
        maths::MovingPlaneConfig,
    };
}

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

/// Create a `rand::rngs::StdRng` from entropy in a normal build, or seeded from 12345 in a test or
/// bench build.
#[cfg(feature = "effect-impls")]
macro_rules! rng {
    () => {{
        use ::rand::{rngs::StdRng, SeedableRng};

        cfg_if::cfg_if! {
            if #[cfg(any(test, feature = "bench"))] {
                StdRng::seed_from_u64(12345)
            } else {
                StdRng::from_entropy()
            }
        }
    }};
}

#[cfg(feature = "effect-impls")]
pub(crate) use rng;

/// A prelude to be imported by effect implementations.
///
/// This module automatically handles `config-impls` and `effect-impls` features.
#[cfg(any(feature = "config-impls", feature = "effect-impls"))]
#[allow(unused_imports)]
pub(crate) mod prelude {
    /// A prelude for the [`EffectConfig`] implementations.
    #[cfg(feature = "config-impls")]
    pub(crate) mod config_prelude {
        pub use crate::traits::EffectConfig;
        pub use effect_proc_macros::Sealed;
        pub use egui::{Align, Layout, RichText, Vec2};
        pub use serde::{Deserialize, Serialize};
    }

    #[cfg(feature = "config-impls")]
    pub use self::config_prelude::*;

    /// A prelude for the [`Effect`] implementations.
    #[cfg(feature = "effect-impls")]
    pub(crate) mod effect_prelude {
        pub(crate) use crate::{
            effects::{rng, sleep},
            traits::Effect,
        };
        pub use effect_proc_macros::BaseEffect;
        pub use glam::Vec3;
        pub use rand::{rngs::StdRng, Rng};
        pub use std::time::Duration;
        pub use tracing::{debug, error, info, instrument, trace, warn};
        pub use tracing_unwrap::{OptionExt, ResultExt};
        pub use ww_driver_trait::Driver;
        pub use ww_frame::{random_vector, Frame3D, FrameObject, FrameType, Object, RGBArray};
    }

    #[cfg(feature = "effect-impls")]
    pub use self::effect_prelude::*;
}

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

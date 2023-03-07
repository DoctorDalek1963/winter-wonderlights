//! This crate provides traits and implementations for various effects, as well as some utility
//! functions.

// Duration is imported and unused for tests and benchmarks because of the sleep macro
#![cfg_attr(any(test, feature = "bench"), allow(unused_imports))]

#[cfg(feature = "impl")]
pub(crate) mod aesthetic;
#[cfg(feature = "impl")]
pub(crate) mod debug;
#[cfg(feature = "impl")]
pub(crate) mod maths;

pub(crate) mod list;
pub(crate) mod traits;

#[cfg(feature = "impl")]
pub use self::{
    aesthetic::LavaLamp,
    debug::{DebugBinaryIndex, DebugOneByOne},
    maths::MovingPlane,
};

pub use self::{
    list::EffectList,
    traits::{save_effect_config_to_file, Effect, EffectConfig},
};

/// Asynchronously sleep for the specified duration and await it when running normally.
///
/// The sleep call gets completely removed for test and bench builds.
#[cfg(feature = "impl")]
macro_rules! sleep {
    ( $dur:expr ) => {
        #[cfg(not(any(test, feature = "bench")))]
        ::tokio::time::sleep($dur).await
    };
}

#[cfg(feature = "impl")]
pub(crate) use sleep;

//! This module provides effects, along with the traits and functions to go with them.

pub(crate) mod aesthetic;
pub(crate) mod debug;
pub(crate) mod list;
pub(crate) mod maths;
pub(crate) mod traits;

pub use self::{
    aesthetic::LavaLamp,
    debug::{DebugBinaryIndex, DebugOneByOne},
    list::EffectList,
    maths::MovingPlane,
    traits::{save_effect_config_to_file, Effect, EffectConfig},
};

//! This module provides lots of effects.

use crate::drivers::Driver;

/// The trait implemented by all effects.
pub trait Effect {
    /// Run the effect with the given driver.
    fn run(&mut self, driver: &mut dyn Driver);
}

mod debug;

pub use self::debug::{DebugBinaryIndex, DebugOneByOne};

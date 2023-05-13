//! This module contains effects for debugging the system.

pub mod binary_index;
pub mod one_by_one;

#[cfg(feature = "effect-impls")]
pub use self::{binary_index::DebugBinaryIndex, one_by_one::DebugOneByOne};

#[cfg(feature = "config-impls")]
pub use self::{binary_index::DebugBinaryIndexConfig, one_by_one::DebugOneByOneConfig};

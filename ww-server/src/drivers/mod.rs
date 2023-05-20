//! This module contains implementations for various drivers. See `Cargo.toml` for the features
//! that enable these drivers and their explanations.

// Thanks to the build script, we are guaranteed to have exactly one of these features enabled.
cfg_if::cfg_if! {
    if #[cfg(feature = "driver-debug")] {
        mod debug;
        pub(super) use debug::DebugDriver as Driver;
    }
}

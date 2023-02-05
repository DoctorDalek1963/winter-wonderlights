//! This crate is designed to display 3D effects on a Christmas tree in real time.
//!
//! The effects are all contained in the [`effects`] module.
//!
//! The drivers are all contained in the [`drivers`] module. These include a driver to debug effects
//! and just write all the data to a log with tracing, as well as a driver to display the effects
//! on a virtual tree, and a driver for real lights. All drivers except the debug one are locked
//! behind crate features.

#![feature(stmt_expr_attributes)]

pub mod drivers;
pub mod effects;
pub mod frame;
pub mod gift_coords;

/// A point in 3D space with f64 values.
pub type PointF = (f64, f64, f64);

cfg_if::cfg_if! {
    if #[cfg(any(test, feature = "bench"))] {
        /// This version of [`sleep`] is only used for tests and benchmarks. It is no-op.
        fn sleep<T>(_: T) {}
    } else {
        use std::time::Duration;

        /// Sleep for the given duration.
        fn sleep(dur: Duration) {
            std::thread::sleep(dur);
        }
    }
}

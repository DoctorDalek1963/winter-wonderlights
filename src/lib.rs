//! This crate is designed to display 3D effects on a Christmas tree in real time.
//!
//! The effects are all contained in the [`effects`] module.
//!
//! The drivers are all contained in the [`drivers`] module. These include a driver to debug effects
//! and just write all the data to a log with tracing, as well as a driver to display the effects
//! on a virtual tree, and a driver for real lights. All drivers except the debug one are locked
//! behind crate features.

#![feature(is_some_and)]
#![feature(stmt_expr_attributes)]
// Duration is imported and unused for tests and benchmarks because of the sleep macro
#![cfg_attr(any(test, feature = "bench"), allow(unused_imports))]

pub mod drivers;
pub mod effects;
pub mod frame;
pub mod gift_coords;

/// A point in 3D space with f64 values.
pub type PointF = (f64, f64, f64);

/// Asynchronously sleep for the specified duration and await it when running normally.
///
/// The sleep call gets completely removed for test and bench builds.
macro_rules! sleep {
    ( $dur:expr ) => {
        #[cfg(not(any(test, feature = "bench")))]
        ::tokio::time::sleep($dur).await
    };
}

pub(crate) use sleep;

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

use std::time::Duration;

/// Sleep for the duration, except when testing. When testing, we don't sleep at all.
#[cfg_attr(any(test, feature = "bench"), allow(unused_variables))]
fn sleep(dur: Duration) {
    #[cfg(not(any(test, feature = "bench")))]
    std::thread::sleep(dur);
}

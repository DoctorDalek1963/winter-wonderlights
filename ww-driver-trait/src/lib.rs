//! This crate provides the [`Driver`] trait and nothing else.
//!
//! It's structured like this to avoid dependency cycles.

use std::sync::OnceLock;
use tracing_unwrap::ResultExt;
use ww_frame::FrameType;

/// A cache for the parsed `LIGHTS_NUM`.
static PARSED_LIGHTS_NUM_CELL: OnceLock<usize> = OnceLock::new();

/// The number of lights provided by the user in the `LIGHTS_NUM` environment variable.
pub fn lights_num() -> usize {
    *PARSED_LIGHTS_NUM_CELL.get_or_init(|| {
        std::env::var("LIGHTS_NUM")
            .expect_or_log("LIGHTS_NUM must be defined")
            .parse::<usize>()
            .expect_or_log("LIGHTS_NUM must be a positive integer")
    })
}

/// The trait implemented by all drivers.
pub trait Driver {
    /// Initialise the driver.
    ///
    /// # Safety
    ///
    /// Some drivers have unsafe initialisations and some require global state, meaning it is
    /// potentially UB to initialise a driver multiple times.
    unsafe fn init() -> Self
    where
        Self: Sized;

    /// Display the given frame using this driver.
    ///
    /// The `max_brightness` argument must be an integer in `0..=100`. This value acts as a
    /// percentage.
    fn display_frame(&mut self, frame: FrameType, max_brightness: u8);
}

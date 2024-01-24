//! This crate provides the [`Driver`] trait and nothing else.
//!
//! It's structured like this to avoid dependency cycles.

#![feature(lint_reasons)]

use std::sync::OnceLock;
use tracing_unwrap::ResultExt;
use ww_frame::FrameType;

/// A [`OnceCell`] to cache the parsed `LIGHTS_NUM`.
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
    fn display_frame(&mut self, frame: FrameType);

    /// Return the number of lights on the chain.
    fn get_lights_count(&self) -> usize;

    /// Clear the display by rendering [`FrameType::Off`].
    fn clear(&mut self) {
        self.display_frame(FrameType::Off);
    }
}

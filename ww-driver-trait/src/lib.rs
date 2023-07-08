//! This crate provides the [`Driver`] trait and nothing else.
//!
//! It's structured like this to avoid dependency cycles.

#![feature(lint_reasons)]

use ww_frame::FrameType;

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

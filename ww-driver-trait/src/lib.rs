//! This crate provides the [`Driver`] trait and nothing else.
//!
//! It's structured like this to avoid dependency cycles.

#![feature(lint_reasons)]

use ww_frame::FrameType;

/// The trait implemented by all drivers.
///
/// Note that for a driver to work in `ww-server`, it's expected to have an initialisation function
/// with the following signature:
/// ```
/// # struct Dummy;
/// # impl Dummy {
/// /// Initialise the driver.
/// pub fn init() -> Self {}
/// # }
/// ```
pub trait Driver {
    /// Display the given frame using this driver.
    fn display_frame(&mut self, frame: FrameType);

    /// Return the number of lights on the chain.
    fn get_lights_count(&self) -> usize;

    /// Clear the display by rendering [`FrameType::Off`].
    fn clear(&mut self) {
        self.display_frame(FrameType::Off);
    }
}

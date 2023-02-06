//! This module provides functionality and implementations for different drivers.

use crate::frame::FrameType;
use tracing::info;

#[cfg(feature = "virtual-tree")]
mod virtual_tree;

#[cfg(feature = "virtual-tree")]
pub use self::virtual_tree::run_effect_on_virtual_tree;

/// The trait implemented by all drivers.
pub trait Driver: Send {
    /// Display the given frame using this driver.
    fn display_frame(&mut self, frame: FrameType);

    /// Return the number of lights on the chain.
    fn get_lights_count(&self) -> usize;

    /// Clear the display by rendering [`FrameType::Off`].
    fn clear(&mut self) {
        self.display_frame(FrameType::Off);
    }
}

/// A simple debug driver that just logs all its input with tracing at the info level.
pub struct DebugDriver {
    pub lights_num: usize,
}

impl Driver for DebugDriver {
    fn display_frame(&mut self, frame: FrameType) {
        info!(?frame);
    }

    fn get_lights_count(&self) -> usize {
        self.lights_num
    }
}

/// A test driver that stores all the frames it receives so that they can be tested.
#[cfg(test)]
pub struct TestDriver {
    pub lights_num: usize,
    pub data: Vec<FrameType>,
}

#[cfg(test)]
impl TestDriver {
    pub fn new(lights_num: usize) -> Self {
        Self {
            lights_num,
            data: vec![],
        }
    }
}

#[cfg(test)]
impl Driver for TestDriver {
    fn display_frame(&mut self, frame: FrameType) {
        self.data.push(frame);
    }

    fn get_lights_count(&self) -> usize {
        self.lights_num
    }
}

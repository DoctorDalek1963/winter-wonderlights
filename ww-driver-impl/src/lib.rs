//! This crate implements various drivers. It has a [`DebugDriver`] available for all builds, and
//! other drivers available behind feature flags.

#![feature(never_type)]
#![feature(is_some_and)]

use tracing::info;
use ww_driver_trait::Driver;
use ww_frame::FrameType;

#[cfg(feature = "virtual-tree")]
mod virtual_tree;

#[cfg(feature = "virtual-tree")]
pub use self::virtual_tree::run_virtual_tree;

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

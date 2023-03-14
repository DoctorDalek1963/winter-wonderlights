//! This module contains implementations for various drivers.

use tracing::info;
use ww_driver_trait::Driver;
use ww_frame::FrameType;

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

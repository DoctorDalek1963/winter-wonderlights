//! This module holds a very simple [`DebugDriver`] to test things with.

use tracing::{info, instrument};
use ww_driver_trait::Driver;
use ww_frame::FrameType;

/// A simple debug driver that just logs all its input with tracing at the info level.
pub struct DebugDriver {
    /// The number of lights used for the debug driver.
    pub lights_num: usize,
}

impl DebugDriver {
    /// Initialise the driver.
    pub fn init() -> Self {
        Self { lights_num: 50 }
    }
}

impl Driver for DebugDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        info!(?frame);
    }

    fn get_lights_count(&self) -> usize {
        self.lights_num
    }
}

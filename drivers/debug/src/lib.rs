//! This crate provides a very simple [`DebugDriver`] to test things with.

use tracing::{info, instrument};
use ww_driver_trait::Driver;
use ww_frame::FrameType;

/// A simple debug driver that just logs all its input with tracing at the info level.
pub struct DebugDriver;

impl Driver for DebugDriver {
    unsafe fn init() -> Self {
        Self
    }

    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType, max_brightness: u8) {
        info!(?frame, ?max_brightness);
    }
}

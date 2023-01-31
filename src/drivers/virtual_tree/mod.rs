//! This module provides implementation for the virtual tree driver.

use crate::{drivers::Driver, frame::FrameType};

/// A simple driver that uses Bevy to create a virtual tree to display the effects on.
pub struct VirtualTreeDriver;

impl Driver for VirtualTreeDriver {
    fn display_frame(&mut self, frame: FrameType) {
        println!("UNFINISHED");
    }

    fn get_lights_count(&self) -> usize {
        500
    }
}

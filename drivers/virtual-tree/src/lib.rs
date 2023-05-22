//! This crate provides a [`VirtualTreeDriver`] for use in `ww-server`.

use std::{process::Command, thread};

use tracing::{debug, instrument};
use ww_driver_trait::Driver;
use ww_frame::FrameType;
use ww_gift_coords::COORDS;

/// The path of the runner binary.
const RUNNER_PATH: &str = env!("CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER");

/// A driver that uses IPC to communicate with Bevy to render a virtual tree.
pub struct VirtualTreeDriver {}

impl VirtualTreeDriver {
    /// Initialise the driver.
    pub fn init() -> Self {
        debug!(?RUNNER_PATH);
        // TODO: Set up IPC
        let socket_path = "/tmp/unnamed.sock";
        thread::spawn(move || {
            Command::new(RUNNER_PATH)
                .arg(socket_path)
                .spawn()
                .expect(&format!("Unable to start runner at path {RUNNER_PATH}"));
        });

        Self {}
    }
}

impl Driver for VirtualTreeDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        tracing::info!(?frame);
        // TODO: Implement this with IPC
        //*CURRENT_FRAME.write().unwrap() = frame;
    }

    fn get_lights_count(&self) -> usize {
        COORDS.lights_num()
    }
}

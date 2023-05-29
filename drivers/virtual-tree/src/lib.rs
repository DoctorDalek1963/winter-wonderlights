//! This crate provides a [`VirtualTreeDriver`] for use in `ww-server`.

use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::{
    io::{self, Write},
    process::Command,
};
use tracing::{debug, error, instrument};
use tracing_unwrap::ResultExt;
use virtual_tree_shared::Message;
use ww_driver_trait::Driver;
use ww_frame::FrameType;
use ww_gift_coords::COORDS;

/// The path of the runner binary.
const RUNNER_PATH: &str = env!("CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER");

/// A driver that uses IPC to communicate with Bevy to render a virtual tree.
pub struct VirtualTreeDriver {
    /// The IPC socket stream to write data to.
    stream: LocalSocketStream,
}

impl VirtualTreeDriver {
    /// Initialise the driver.
    #[instrument]
    pub fn init() -> Self {
        debug!(?RUNNER_PATH);

        let socket_path = match NameTypeSupport::query() {
            NameTypeSupport::OnlyPaths => {
                concat!(env!("DATA_DIR"), "/winter-wonderlights-virtual-tree.sock")
            }
            NameTypeSupport::OnlyNamespaced | NameTypeSupport::Both => {
                "@winter-wonderlights-virtual-tree.sock"
            }
        };

        let socket_listener = match LocalSocketListener::bind(socket_path) {
            Ok(x) => x,
            Err(e) => {
                if e.kind() == io::ErrorKind::AddrInUse {
                    panic!("Expected for path {RUNNER_PATH:?} to be usable as the socket");
                } else {
                    error!(?e, "Unknown error");
                    panic!(
                        "Unexpected error trying to bind socket to {RUNNER_PATH:?} error: {e:?}"
                    );
                }
            }
        };

        Command::new(RUNNER_PATH)
            .arg(socket_path)
            .spawn()
            .expect_or_log(&format!("Unable to start runner at path {RUNNER_PATH}"));

        let stream = socket_listener
            .accept()
            .expect_or_log("The runner should successfully connect to the driver's socket");

        Self { stream }
    }
}

impl Driver for VirtualTreeDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        debug!(?frame, "Writing frame to socket");

        self.stream
            .write(
                &bincode::serialize(&Message::UpdateFrame(frame))
                    .expect("Serializing a Message should not fail"),
            )
            .expect_or_log("Failed to write to the socket");
    }

    fn get_lights_count(&self) -> usize {
        COORDS.lights_num()
    }
}

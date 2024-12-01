//! This crate provides a [`VirtualTreeDriver`] for use in `ww-server`.

use interprocess_docfix::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::{
    io::{self, Write},
    process::Command,
};
use tracing::{debug, error, instrument, trace};
use tracing_unwrap::ResultExt;
use virtual_tree_shared::Message;
use ww_driver_trait::Driver;
use ww_frame::FrameType;

/// Get the path of the runner binary.
///
/// We try to read the `CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER` environment variable at runtime if it's
/// available (since Nix needs to overwrite this path), but we default to the compile-time version
/// if it's not available at runtime.
fn get_runner_path() -> String {
    std::env::var("CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER")
        .unwrap_or(env!("CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER").to_owned())
}

/// A driver that uses IPC to communicate with Bevy to render a virtual tree.
pub struct VirtualTreeDriver {
    /// The IPC socket stream to write data to.
    stream: LocalSocketStream,
}

impl Driver for VirtualTreeDriver {
    #[instrument]
    unsafe fn init() -> Self {
        let runner_path = get_runner_path();

        debug!(?runner_path);

        let socket_path = match NameTypeSupport::query() {
            NameTypeSupport::OnlyPaths => {
                format!(
                    "{}/winter-wonderlights-virtual-tree.sock",
                    std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
                )
            }
            NameTypeSupport::OnlyNamespaced | NameTypeSupport::Both => {
                "@winter-wonderlights-virtual-tree.sock".to_owned()
            }
        };

        let socket_listener = match LocalSocketListener::bind(&socket_path[..]) {
            Ok(x) => x,
            Err(e) => {
                if e.kind() == io::ErrorKind::AddrInUse {
                    panic!("Expected for path {runner_path:?} to be usable as the socket");
                } else {
                    error!(?e, "Unknown error");
                    panic!(
                        "Unexpected error trying to bind socket to {runner_path:?} error: {e:?}"
                    );
                }
            }
        };

        Command::new(&runner_path)
            .arg(socket_path)
            .spawn()
            .expect_or_log(&format!("Unable to start runner at path {runner_path}"));

        let stream = socket_listener
            .accept()
            .expect_or_log("The runner should successfully connect to the driver's socket");

        Self { stream }
    }

    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType, max_brightness: u8) {
        trace!(?frame, "Writing frame to socket");

        self.stream
            .write(
                &bincode::serialize(&Message::UpdateFrame(frame, max_brightness))
                    .expect_or_log("Serializing a Message should not fail"),
            )
            .expect_or_log("Failed to write to the socket");
    }
}

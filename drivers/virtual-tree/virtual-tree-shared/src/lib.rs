//! This crate provides a shared [`Message`] type for the virtual tree driver and runner to use.

use serde::{Deserialize, Serialize};
use ww_frame::FrameType;

/// A message for the driver to send to the runner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    /// Update the current frame.
    UpdateFrame(FrameType),

    /// Shut down the runner.
    Shutdown,
}

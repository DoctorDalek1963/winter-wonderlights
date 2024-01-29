//! This module contains implementations for various drivers. See `Cargo.toml` for the features
//! that enable these drivers and their explanations.

use std::ops::{Deref, DerefMut};
use tracing::{error, info, instrument};
use ww_driver_trait::Driver;
use ww_frame::FrameType;

// Thanks to the build script, we are guaranteed to have exactly one of these features enabled.
cfg_if::cfg_if! {
    if #[cfg(feature = "driver-debug")] {
        use debug::DebugDriver as DriverImpl;
    } else if #[cfg(feature = "driver-raspi-ws2811")] {
        use raspi_ws2811::Ws2811Driver as DriverImpl;
    } else {
        compile_error!("You must enable exactly one driver feature");
    }
}

/// A transparent wrapper around the `Driver` trait implementation. This wrapper clears the tree
/// when dropped by calling:
/// ```rust
/// # impl Drop for DriverWrapper {
/// # fn drop(&mut self) {
/// self.display_frame(FrameType::Off);
/// # } }
/// ```
pub(super) struct DriverWrapper(DriverImpl);

impl Deref for DriverWrapper {
    type Target = DriverImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DriverWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for DriverWrapper {
    #[instrument(skip_all)]
    fn drop(&mut self) {
        info!("Dropping DriverWrapper");

        if std::thread::panicking() {
            error!("Thread is panicking. Unable to blank out tree");
        } else {
            self.display_frame(FrameType::Off);
        }
    }
}

impl Driver for DriverWrapper {
    /// Initialise the driver.
    ///
    /// # Safety
    ///
    /// For most drivers, it is undefined behaviour to initialise the driver multiple times. You
    /// must ensure that this method is called at most once.
    unsafe fn init() -> Self {
        #[allow(
            clippy::undocumented_unsafe_blocks,
            reason = "this is explicitly not safe, for the reasons described above"
        )]
        unsafe {
            Self(DriverImpl::init())
        }
    }

    #[inline]
    fn display_frame(&mut self, frame: FrameType) {
        self.0.display_frame(frame);
    }
}

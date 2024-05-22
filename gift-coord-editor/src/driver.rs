//! This module provides [`EditorDriver`], which is used for the [`super::Command::Light`] command
//! when a driver is enabled.

#[cfg(feature = "driver-raspi-ws2811")]
use raspi_ws2811::Ws2811Driver;

#[cfg(feature = "_driver")]
use ww_driver_trait::Driver;

/// The driver for the editor.
pub struct EditorDriver {
    /// Use WS2811 lights on a Raspberry Pi.
    #[cfg(feature = "driver-raspi-ws2811")]
    inner: Ws2811Driver,
}

impl EditorDriver {
    /// Initialise the driver for the editor.
    ///
    /// # Safety
    ///
    /// This method should only be called once. See [`ww_driver_trait::Driver::init`].
    pub unsafe fn init() -> Self {
        #[cfg(not(feature = "_driver"))]
        return Self {};

        #[cfg(feature = "driver-raspi-ws2811")]
        return Self {
            inner: unsafe { Ws2811Driver::init() },
        };
    }

    #[cfg(feature = "_driver")]
    pub fn enable_one_light(&mut self, idx: usize, total_lights: usize) {
        if idx >= total_lights {
            eprintln!("ERROR: Requested index {idx} is out of bounds for {total_lights} lights");
            return;
        }

        let mut v = vec![[0; 3]; total_lights];
        v[idx] = [255; 3];
        self.inner
            .display_frame(ww_frame::FrameType::RawData(v), 255);
    }
}

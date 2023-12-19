//! This crate provides a driver to run WS2811 RGB LEDs on a Raspberry Pi.
//!
//! See <https://pdf1.alldatasheet.com/datasheet-pdf/view/1132633/WORLDSEMI/WS2811.html> for the
//! datasheet and <https://docs.rs/rs_ws281x/0.4.4/rs_ws281x> for the backend used to power this
//! driver.

#![feature(lint_reasons)]

use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder, StripType};
use tracing::{info, instrument};
use tracing_unwrap::ResultExt;
use ww_driver_trait::Driver;
use ww_frame::FrameType;
use ww_gift_coords::COORDS;

/// The frequency of the signal to the LEDs in Hz.
///
/// 800,000 is a good default but it should never go below 400,000.
const FREQUENCY: u32 = 800_000;

/// The channel number for DMA.
///
/// 10 is a good default but you MUST AVOID 0, 1, 2, 3, 5, 6, or 7.
/// Make sure this DMA channel is not already in use.
const DMA_CHANNEL_NUMBER: i32 = 10;

/// The GPIO pin number of the pin to send data down.
///
/// 18 is a good default but this can be any pin which is capable of any of PCM, PWM, or SPI. See
/// <https://pinout.xyz> for details on which pins support these.
const GPIO_PIN_NUMBER: i32 = 18;

/// The type of the LED strip.
///
/// `Ws2811Rgb` is a good default but see
/// <https://docs.rs/rs_ws281x/0.4.4/rs_ws281x/enum.StripType.html> for all the options.
const STRIP_TYPE: StripType = StripType::Ws2811Rgb;

/// A driver that can run WS2811 RGB LEDs on a Raspberry Pi.
pub struct Ws2811Driver {
    /// The internal controller for the LEDs.
    controller: Controller,
}

impl Ws2811Driver {
    /// Display the given RGB colours on the lights.
    #[instrument(skip_all)]
    fn display_colours(&mut self, colours: &[[u8; 3]]) {
        let leds = self.controller.leds_mut(0);

        for (idx, &[r, g, b]) in colours.iter().enumerate() {
            if let Some(colour) = leds.get_mut(idx) {
                *colour = [b, g, r, 0];
            } else {
                break;
            }
        }

        self.controller
            .render()
            .expect_or_log("Should be able to render LEDs through controller");
    }
}

impl Driver for Ws2811Driver {
    #[instrument]
    unsafe fn init() -> Self {
        let controller = ControllerBuilder::new()
            .freq(FREQUENCY)
            .dma(DMA_CHANNEL_NUMBER)
            .channel(
                0,
                ChannelBuilder::new()
                    .pin(GPIO_PIN_NUMBER)
                    .count(COORDS.lights_num() as _)
                    .strip_type(STRIP_TYPE)
                    .brightness(255)
                    .build(),
            )
            .build()
            .expect_or_log("Failed to build controller for raspi-ws2811 driver");

        info!("Initted ws281x controller");

        Self { controller }
    }

    fn display_frame(&mut self, frame: FrameType) {
        let colours = match frame {
            FrameType::Off => vec![[0; 3]; self.get_lights_count()],
            FrameType::RawData(data) => data,
            FrameType::Frame3D(frame) => frame.to_raw_data(),
        };
        self.display_colours(&colours);
    }

    #[inline]
    fn get_lights_count(&self) -> usize {
        COORDS.lights_num()
    }
}

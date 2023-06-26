//! This crate provides a driver to run WS2811 RGB LEDs on a Raspberry Pi.
//!
//! See <https://pdf1.alldatasheet.com/datasheet-pdf/view/1132633/WORLDSEMI/WS2811.html> for the
//! datasheet.

#![feature(lint_reasons)]

use rppal::gpio::{Error, Gpio, OutputPin};
use std::{thread, time::Duration};
use tracing::{error, instrument};
use ww_driver_trait::Driver;
use ww_frame::FrameType;
use ww_gift_coords::COORDS;

/// A driver that can run WS2811 RGB LEDs on a Raspberry Pi.
pub struct Ws2811Driver {
    /// The pin to output data on.
    pin: OutputPin,
}

// TODO: Allow specifying things like pin number and high/low speed mode in build.rs
impl Ws2811Driver {
    /// Initialise the lights on GPIO pin 18, assuming high speed mode.
    #[instrument]
    pub fn init() -> Self {
        macro_rules! handle_error {
            ($error:ident) => {{
                match $error {
                    Error::UnknownModel => error!("Unknown model. driver-raspi-ws2811 only works on Raspberry Pis which are supported by rppal. Is this a Raspberry Pi?"),
                    Error::PinUsed(_) | Error::PinNotAvailable(_) => error!("Pin unavailable. Please free up GPIO pin 18 and try again"),
                    Error::PermissionDenied(msg) => error!(?msg, "Permission denied. Please enable GPIO pin access for the current user or run as root"),
                    error => error!(?error, "Unknown error when initialising driver"),
                };
                panic!("Error when initialising driver. See logs for more info.");
            }};
        }

        let gpio = match Gpio::new() {
            Ok(gpio) => gpio,
            Err(error) => handle_error!(error),
        };

        let pin = match gpio.get(18) {
            Ok(pin) => pin.into_output_low(),
            Err(error) => handle_error!(error),
        };

        Self { pin }
    }

    /// Send a 0 bit down the data line.
    fn send_0_bit(&mut self) {
        self.pin.set_high();
        thread::sleep(Duration::from_nanos(250));
        self.pin.set_low();
        thread::sleep(Duration::from_nanos(1000));
    }

    /// Send a 1 bit down the data line.
    fn send_1_bit(&mut self) {
        self.pin.set_high();
        thread::sleep(Duration::from_nanos(600));
        self.pin.set_low();
        thread::sleep(Duration::from_nanos(650));
    }

    /// Send a reset signal down the data line.
    fn send_reset(&mut self) {
        self.pin.set_low();
        thread::sleep(Duration::from_micros(51));
    }

    /// Send the given bits, highest bit first.
    fn send_bits(&mut self, bits: u8) {
        macro_rules! send_mask {
            ($mask:literal) => {
                if bits & $mask == $mask {
                    self.send_1_bit();
                } else {
                    self.send_0_bit();
                }
            };
        }

        send_mask!(0b10000000);
        send_mask!(0b01000000);
        send_mask!(0b00100000);
        send_mask!(0b00010000);
        send_mask!(0b00001000);
        send_mask!(0b00000100);
        send_mask!(0b00000010);
        send_mask!(0b00000001);
    }

    /// Send the RGB colour down the data line.
    fn send_rgb(&mut self, [red, green, blue]: [u8; 3]) {
        self.send_bits(red);
        self.send_bits(green);
        self.send_bits(blue);
    }

    /// Send the given colours and then send the reset signal.
    fn send_colours_and_reset(&mut self, colours: &[[u8; 3]]) {
        for rgb in colours {
            self.send_rgb(*rgb);
        }
        self.send_reset();
    }
}

impl Driver for Ws2811Driver {
    fn display_frame(&mut self, frame: FrameType) {
        let colours = match frame {
            FrameType::Off => vec![[0; 3]; self.get_lights_count()],
            FrameType::RawData(data) => data,
            FrameType::Frame3D(frame) => frame.to_raw_data(),
        };
        self.send_colours_and_reset(&colours);
    }

    #[inline]
    fn get_lights_count(&self) -> usize {
        COORDS.lights_num()
    }
}

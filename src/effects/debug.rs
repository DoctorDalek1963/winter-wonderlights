//! This module provides some simple debug effects.

use serde::{Deserialize, Serialize};

use crate::{
    drivers::Driver,
    effects::Effect,
    frame::{FrameType, RGBTuple},
    sleep,
};
use std::time::Duration;

/// Light up each light individually, one-by-one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebugOneByOne {
    /// The number of milliseconds that the light is on for.
    light_time_ms: u64,

    /// The number of milliseconds to wait after turning off all the lights.
    dark_time_ms: u64,

    /// The color for the current light.
    color: RGBTuple,
}

impl Effect for DebugOneByOne {
    fn effect_name() -> &'static str {
        "DebugOneByOne"
    }

    fn default() -> Self {
        Self {
            light_time_ms: 1000,
            dark_time_ms: 100,
            color: (255, 255, 255),
        }
    }

    fn run(&mut self, driver: &mut dyn Driver) {
        driver.clear();

        let count = driver.get_lights_count();
        let mut data = vec![(0, 0, 0); count];

        // Display the color on each LED, then blank it, pausing between each one.
        for i in 0..count {
            data[i] = self.color;
            driver.display_frame(FrameType::RawData(data.clone()));
            data[i] = (0, 0, 0);
            sleep(Duration::from_millis(self.light_time_ms));

            driver.clear();
            sleep(Duration::from_millis(self.dark_time_ms));
        }
    }
}

/// Make each light flash its index in binary, using blue for 1 and red for 0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebugBinaryIndex {
    /// The number of milliseconds that the lights are on for.
    light_time_ms: u64,

    /// The number of milliseconds to wait after turning off all the lights.
    dark_time_ms: u64,

    /// The color to illuminate lights representing 0.
    zero_color: RGBTuple,

    /// The color to illuminate lights representing 1.
    one_color: RGBTuple,
}

impl Effect for DebugBinaryIndex {
    fn effect_name() -> &'static str {
        "DebugBinaryIndex"
    }

    fn default() -> Self {
        Self {
            light_time_ms: 1500,
            dark_time_ms: 500,
            zero_color: (255, 0, 0),
            one_color: (0, 0, 255),
        }
    }

    fn run(&mut self, driver: &mut dyn Driver) {
        driver.clear();

        // Get the simple binary versions of the index of each number in the range
        let binary_index_vecs: Vec<Vec<char>> = (0..driver.get_lights_count())
            .map(|n| format!("{n:b}").chars().collect())
            .collect();

        // We need to pad each number to the same length, so we find the maxmimum length
        let binary_number_length = binary_index_vecs
            .last()
            .expect("There should be at least one light")
            .len();

        // Now we pad out all the elements and convert them to colours
        let colours_for_each_light: Vec<Vec<RGBTuple>> = binary_index_vecs
            .into_iter()
            .map(|nums: Vec<char>| -> Vec<RGBTuple> {
                // This vec has the right length, so we just have to copy the actual numbers into
                // the end of it.
                let mut v = vec!['0'; binary_number_length];
                v[binary_number_length - nums.len()..].copy_from_slice(&nums);

                // Now map each number char to a colour
                v.into_iter()
                    .map(|c| match c {
                        '0' => self.zero_color,
                        '1' => self.one_color,
                        _ => unreachable!("Binary numbers should only contain '0' and '1'"),
                    })
                    .collect()
            })
            .collect();

        assert!(
            colours_for_each_light
                .iter()
                .map(Vec::len)
                .all(|n| n == binary_number_length),
            "Every Vec<RGBTuple> in the list must be the same length"
        );

        // Now actually display the colours on the lights
        for i in 0..binary_number_length {
            let colours_at_idx: Vec<RGBTuple> =
                colours_for_each_light.iter().map(|cols| cols[i]).collect();

            driver.display_frame(FrameType::RawData(colours_at_idx.clone()));
            sleep(Duration::from_millis(self.light_time_ms));

            driver.clear();
            sleep(Duration::from_millis(self.dark_time_ms));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::TestDriver;

    #[test]
    fn debug_one_by_one_test() {
        let mut driver = TestDriver::new(5);
        DebugOneByOne::default().run(&mut driver);

        #[rustfmt::skip]
        assert_eq!(
            driver.data,
            vec![
                FrameType::Off,
                FrameType::RawData(vec![(255, 255, 255), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0)]),
                FrameType::Off,
                FrameType::RawData(vec![(0, 0, 0), (255, 255, 255), (0, 0, 0), (0, 0, 0), (0, 0, 0)]),
                FrameType::Off,
                FrameType::RawData(vec![(0, 0, 0), (0, 0, 0), (255, 255, 255), (0, 0, 0), (0, 0, 0)]),
                FrameType::Off,
                FrameType::RawData(vec![(0, 0, 0), (0, 0, 0), (0, 0, 0), (255, 255, 255), (0, 0, 0)]),
                FrameType::Off,
                FrameType::RawData(vec![(0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (255, 255, 255)]),
                FrameType::Off,
            ]
        );
    }

    #[test]
    fn debug_binary_index_test() {
        let mut driver = TestDriver::new(8);
        DebugBinaryIndex::default().run(&mut driver);

        #[rustfmt::skip]
        assert_eq!(
            driver.data,
            vec![
                FrameType::Off,
                FrameType::RawData(vec![
                    (255, 0, 0), (255, 0, 0), (255, 0, 0), (255, 0, 0),
                    (0, 0, 255), (0, 0, 255), (0, 0, 255), (0, 0, 255)
                ]),
                FrameType::Off,
                FrameType::RawData(vec![
                    (255, 0, 0), (255, 0, 0), (0, 0, 255), (0, 0, 255),
                    (255, 0, 0), (255, 0, 0), (0, 0, 255), (0, 0, 255)
                ]),
                FrameType::Off,
                FrameType::RawData(vec![
                    (255, 0, 0), (0, 0, 255), (255, 0, 0), (0, 0, 255),
                    (255, 0, 0), (0, 0, 255), (255, 0, 0), (0, 0, 255)
                ]),
                FrameType::Off,
            ]
        );
    }
}

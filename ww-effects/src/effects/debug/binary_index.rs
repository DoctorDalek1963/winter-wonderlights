//! This crate provides the [`DebugBinaryIndex`] effect.

#[cfg(feature = "config-impls")]
pub use config::DebugBinaryIndexConfig;

#[cfg(feature = "effect-impls")]
pub use effect::DebugBinaryIndex;

use crate::effects::prelude::*;

#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the binary index effect; includes timing and colors.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Sealed)]
    pub struct DebugBinaryIndexConfig {
        /// The number of milliseconds that the lights are on for.
        pub light_time_ms: u64,

        /// The number of milliseconds to wait after turning off all the lights.
        pub dark_time_ms: u64,

        /// The color to illuminate lights representing 0.
        pub zero_color: [u8; 3],

        /// The color to illuminate lights representing 1.
        pub one_color: [u8; 3],
    }

    impl Default for DebugBinaryIndexConfig {
        fn default() -> Self {
            Self {
                light_time_ms: 1500,
                dark_time_ms: 500,
                zero_color: [255, 0, 0],
                one_color: [0, 0, 255],
            }
        }
    }

    impl EffectConfig for DebugBinaryIndexConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            ui.label(RichText::new("DebugBinaryIndex config").heading());
            ui.add_space(UI_SPACING);

            let mut config_changed = false;

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.light_time_ms, 0..=1500)
                        .suffix("ms")
                        .text("Light time"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.dark_time_ms, 0..=1500)
                        .suffix("ms")
                        .text("Dark time"),
                )
                .changed();

            ui.add_space(UI_SPACING);

            config_changed |= colour_picker(ui, &mut self.zero_color, "Zero colour").changed();
            config_changed |= colour_picker(ui, &mut self.one_color, "One colour").changed();

            ui.add_space(UI_SPACING);

            if ui.button("Reset to defaults").clicked() {
                *self = Self::default();
                config_changed = true;
            }

            config_changed
        }
    }
}

#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;

    /// Make each light flash its index in binary.
    #[derive(Clone, Debug, Default, PartialEq, BaseEffect)]
    pub struct DebugBinaryIndex {
        /// The config for this effect.
        config: DebugBinaryIndexConfig,
    }

    impl Effect for DebugBinaryIndex {
        type Config = DebugBinaryIndexConfig;

        async fn run(self, driver: &mut dyn Driver) {
            driver.clear();

            // Get the simple binary versions of the index of each number in the range
            let binary_index_vecs: Vec<Vec<char>> = (0..driver.get_lights_count())
                .map(|n| format!("{n:b}").chars().collect())
                .collect();

            // We need to pad each number to the same length, so we find the maxmimum length
            let binary_number_length = binary_index_vecs
                .last()
                .expect_or_log("There should be at least one light")
                .len();

            // Now we pad out all the elements and convert them to colours
            let colours_for_each_light: Vec<Vec<RGBArray>> = binary_index_vecs
                .into_iter()
                .map(|nums: Vec<char>| -> Vec<RGBArray> {
                    // This vec has the right length, so we just have to copy the actual numbers into
                    // the end of it.
                    let mut v = vec!['0'; binary_number_length];
                    v[binary_number_length - nums.len()..].copy_from_slice(&nums);

                    // Now map each number char to a colour
                    v.into_iter()
                        .map(|c| match c {
                            '0' => self.config.zero_color,
                            '1' => self.config.one_color,
                            _ => unreachable!("Binary numbers should only contain '0' and '1'"),
                        })
                        .collect()
                })
                .collect();

            // Now actually display the colours on the lights
            for i in 0..binary_number_length {
                let colours_at_idx: Vec<RGBArray> = colours_for_each_light
                    .iter()
                    .map(|colours_for_this_light| colours_for_this_light[i])
                    .collect();

                driver.display_frame(FrameType::RawData(colours_at_idx));
                sleep!(Duration::from_millis(self.config.light_time_ms));

                driver.clear();
                sleep!(Duration::from_millis(self.config.dark_time_ms));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};
    use ww_frame::FrameType;

    #[tokio::test]
    async fn debug_binary_index_test() {
        let mut driver = TestDriver::new(8);
        DebugBinaryIndex::default().run(&mut driver).await;

        #[rustfmt::skip]
        assert_eq!(
            driver.data,
            vec![
                FrameType::Off,
                FrameType::RawData(vec![
                    [255, 0, 0], [255, 0, 0], [255, 0, 0], [255, 0, 0],
                    [0, 0, 255], [0, 0, 255], [0, 0, 255], [0, 0, 255]
                ]),
                FrameType::Off,
                FrameType::RawData(vec![
                    [255, 0, 0], [255, 0, 0], [0, 0, 255], [0, 0, 255],
                    [255, 0, 0], [255, 0, 0], [0, 0, 255], [0, 0, 255]
                ]),
                FrameType::Off,
                FrameType::RawData(vec![
                    [255, 0, 0], [0, 0, 255], [255, 0, 0], [0, 0, 255],
                    [255, 0, 0], [0, 0, 255], [255, 0, 0], [0, 0, 255]
                ]),
                FrameType::Off,
            ]
        );
    }
}

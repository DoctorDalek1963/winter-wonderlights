//! This crate provides the [`DebugBinaryIndex`] effect.

#[cfg(feature = "config-impls")]
pub use config::DebugBinaryIndexConfig;

#[cfg(feature = "effect-impls")]
pub use effect::DebugBinaryIndex;

#[cfg(feature = "config-impls")]
mod config {
    use crate::traits::EffectConfig;
    use effect_proc_macros::Sealed;
    use egui::{Align, Layout, RichText, Vec2};
    use serde::{Deserialize, Serialize};

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

            ui.allocate_ui_with_layout(
                Vec2::splat(0.),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label("Zero color: ");
                    config_changed |= ui.color_edit_button_srgb(&mut self.zero_color).changed();

                    ui.label("One color: ");
                    config_changed |= ui.color_edit_button_srgb(&mut self.one_color).changed();
                },
            );

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
    use crate::{
        effects::sleep,
        traits::{Effect, EffectConfig},
    };
    use async_trait::async_trait;
    use effect_proc_macros::BaseEffect;
    use std::time::Duration;
    use tracing::trace;
    use ww_driver_trait::Driver;
    use ww_frame::{FrameType, RGBArray};

    use super::config::DebugBinaryIndexConfig;

    /// Make each light flash its index in binary.
    #[derive(Clone, Debug, Default, PartialEq, BaseEffect)]
    pub struct DebugBinaryIndex {
        /// The config for this effect.
        config: DebugBinaryIndexConfig,
    }

    #[async_trait]
    impl Effect for DebugBinaryIndex {
        type Config = DebugBinaryIndexConfig;

        async fn run(self, driver: &mut dyn Driver) {
            #[derive(Debug)]
            enum Binary {
                Zero,
                One,
            }

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
            let binary_for_each_light: Vec<Vec<Binary>> = binary_index_vecs
                .into_iter()
                .map(|nums: Vec<char>| -> Vec<Binary> {
                    // This vec has the right length, so we just have to copy the actual numbers into
                    // the end of it.
                    let mut v = vec!['0'; binary_number_length];
                    v[binary_number_length - nums.len()..].copy_from_slice(&nums);

                    // Now map each number char to a colour
                    v.into_iter()
                        .map(|c| match c {
                            '0' => Binary::Zero,
                            '1' => Binary::One,
                            _ => unreachable!("Binary numbers should only contain '0' and '1'"),
                        })
                        .collect()
                })
                .collect();

            assert!(
                binary_for_each_light
                    .iter()
                    .map(Vec::len)
                    .all(|n| n == binary_number_length),
                "Every Vec<RGBTuple> in the list must be the same length"
            );

            trace!(?binary_for_each_light);

            // Now actually display the colours on the lights
            for i in 0..binary_number_length {
                let colours_at_idx: Vec<RGBArray> = binary_for_each_light
                    .iter()
                    .map(|cols| match cols[i] {
                        Binary::Zero => self.config.zero_color,
                        Binary::One => self.config.one_color,
                    })
                    .collect();

                driver.display_frame(FrameType::RawData(colours_at_idx.clone()));
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

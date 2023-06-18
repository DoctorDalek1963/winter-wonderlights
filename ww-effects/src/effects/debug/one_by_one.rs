//! This crate provides the [`DebugOneByOne`] effect.

#[cfg(feature = "config-impls")]
pub use config::DebugOneByOneConfig;

#[cfg(feature = "effect-impls")]
pub use effect::DebugOneByOne;

use crate::effects::prelude::*;

#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the `DebugOneByOne` effect; includes timing and the color.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Sealed)]
    pub struct DebugOneByOneConfig {
        /// The number of milliseconds that the light is on for.
        pub light_time_ms: u64,

        /// The number of milliseconds to wait after turning off all the lights.
        pub dark_time_ms: u64,

        /// The color for the current light.
        pub colour: [u8; 3],
    }

    impl Default for DebugOneByOneConfig {
        fn default() -> Self {
            Self {
                light_time_ms: 1000,
                dark_time_ms: 100,
                colour: [255, 255, 255],
            }
        }
    }

    impl EffectConfig for DebugOneByOneConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            ui.label(RichText::new("DebugOneByOne config").heading());
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

            config_changed |= colour_picker(ui, &mut self.colour, "Colour").changed();

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

    /// Light up each light individually, one-by-one.
    #[derive(Clone, Debug, Default, PartialEq, BaseEffect)]
    pub struct DebugOneByOne {
        /// The config for this effect.
        config: DebugOneByOneConfig,
    }

    impl Effect for DebugOneByOne {
        async fn run(self, driver: &mut dyn Driver) {
            driver.clear();

            let count = driver.get_lights_count();
            let mut data = vec![[0, 0, 0]; count];

            // Display the color on each LED, then blank it, pausing between each one.
            for i in 0..count {
                data[i] = self.config.colour;
                driver.display_frame(FrameType::RawData(data.clone()));
                data[i] = [0, 0, 0];
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
    async fn debug_one_by_one_test() {
        let mut driver = TestDriver::new(5);
        DebugOneByOne::default().run(&mut driver).await;

        #[rustfmt::skip]
        assert_eq!(
            driver.data,
            vec![
                FrameType::Off,
                FrameType::RawData(vec![[255, 255, 255], [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0]]),
                FrameType::Off,
                FrameType::RawData(vec![[0, 0, 0], [255, 255, 255], [0, 0, 0], [0, 0, 0], [0, 0, 0]]),
                FrameType::Off,
                FrameType::RawData(vec![[0, 0, 0], [0, 0, 0], [255, 255, 255], [0, 0, 0], [0, 0, 0]]),
                FrameType::Off,
                FrameType::RawData(vec![[0, 0, 0], [0, 0, 0], [0, 0, 0], [255, 255, 255], [0, 0, 0]]),
                FrameType::Off,
                FrameType::RawData(vec![[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0], [255, 255, 255]]),
                FrameType::Off,
            ]
        );
    }
}

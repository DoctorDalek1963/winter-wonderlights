//! This crate provides the [`DebugOneByOne`] effect.

#[cfg(feature = "config-impls")]
pub use config::DebugOneByOneConfig;

#[cfg(feature = "effect-impls")]
pub use effect::DebugOneByOne;

use crate::effects::prelude::*;

/// Contains the config for the [`DebugOneByOne`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`DebugOneByOne`] effect; includes timing and colour.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
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

            config_changed
        }
    }
}

/// Contains the [`DebugOneByOne`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use ww_gift_coords::COORDS;

    /// Light up each light individually, one-by-one.
    #[derive(Clone, Debug, PartialEq, Eq, BaseEffect)]
    pub struct DebugOneByOne {
        /// Which index should we be displaying next frame?
        index: usize,

        /// Should the lights be on or off in the next frame?
        on: bool,
    }

    impl Effect for DebugOneByOne {
        fn from_config(_config: DebugOneByOneConfig) -> Self {
            Self { index: 0, on: true }
        }

        fn next_frame(&mut self, config: &DebugOneByOneConfig) -> Option<(FrameType, Duration)> {
            let ret_val = if self.on {
                debug_assert!(
                    self.index < COORDS.lights_num(),
                    "The state machine should never let self.index get out of bounds"
                );

                let mut frame_data = vec![[0; 3]; COORDS.lights_num()];
                frame_data[self.index] = config.colour;

                // The next frame will light up the next light
                self.index += 1;

                (
                    FrameType::RawData(frame_data),
                    Duration::from_millis(config.light_time_ms),
                )
            } else {
                // If the previous frame was the final one, we don't want an extra pause before the
                // server restarts this effect
                if self.index == COORDS.lights_num() {
                    return None;
                }

                (FrameType::Off, Duration::from_millis(config.dark_time_ms))
            };

            self.on = !self.on;
            Some(ret_val)
        }

        #[cfg(any(test, feature = "bench"))]
        fn loops_to_test() -> Option<NonZeroU16> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot_effect;

    #[test]
    fn debug_one_by_one_test() {
        snapshot_effect!(DebugOneByOne);
    }
}

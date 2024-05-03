//! This crate provides the [`DebugBinaryIndex`] effect.

#[cfg(feature = "config-impls")]
pub use config::DebugBinaryIndexConfig;

#[cfg(feature = "effect-impls")]
pub use effect::DebugBinaryIndex;

use crate::effects::prelude::*;

/// Contains the config for the [`DebugBinaryIndex`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`DebugBinaryIndex`] effect; includes timing and colors.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
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

            config_changed
        }
    }
}

/// Contains the [`DebugBinaryIndex`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use bitvec::vec::BitVec;
    use ww_gift_coords::COORDS;

    /// Make each light flash its index in binary.
    #[derive(Clone, Debug, PartialEq, Eq, BaseEffect)]
    pub struct DebugBinaryIndex {
        /// The list of patterns to display, *in reverse order*.
        ///
        /// Each [`BitVec`] says whether each light should be on or off this frame, and the [`Vec`]
        /// contains the [`BitVec`]s in reverse order, so [`Vec::pop`] should give the next frame in
        /// the sequence.
        patterns: Vec<BitVec>,

        /// Should the lights be on or off in the next frame?
        on: bool,
    }

    impl Effect for DebugBinaryIndex {
        fn from_config(_config: DebugBinaryIndexConfig) -> Self {
            let binary_number_length =
                f32::floor(f32::log2((COORDS.lights_num() - 1) as f32)) as usize + 1;
            debug_assert_eq!(
                format!("{:b}", COORDS.lights_num() - 1).len(),
                binary_number_length,
                "binary_number_length should be correct as calculated from the logarithm"
            );

            let binary_numbers: Vec<String> = (0..COORDS.lights_num())
                .map(|idx| format!("{idx:0>binary_number_length$b}"))
                .collect();

            let patterns = (0..binary_number_length)
                .rev()
                .map(|num_idx| {
                    binary_numbers
                        .iter()
                        .map(|bin_str| {
                            bin_str.chars().nth(num_idx).expect_or_log(
                                "Every string in binary_numbers should have enough padding",
                            ) == '1'
                        })
                        .collect()
                })
                .collect();

            Self { patterns, on: true }
        }

        fn next_frame(&mut self, config: &DebugBinaryIndexConfig) -> Option<(FrameType, Duration)> {
            let ret_val = if self.on {
                let pattern = self.patterns.pop()?;

                debug_assert_eq!(
                    pattern.len(),
                    COORDS.lights_num(),
                    "We should have as many bits as there are lights on the tree"
                );

                let frame_data = pattern
                    .into_iter()
                    .map(|one| {
                        if one {
                            config.one_color
                        } else {
                            config.zero_color
                        }
                    })
                    .collect();

                (
                    FrameType::RawData(frame_data),
                    Duration::from_millis(config.light_time_ms),
                )
            } else {
                // If the previous frame was the final one, we don't want an extra pause before the
                // server restarts this effect
                if self.patterns.is_empty() {
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
    fn debug_binary_index_test() {
        snapshot_effect!(DebugBinaryIndex);
    }
}

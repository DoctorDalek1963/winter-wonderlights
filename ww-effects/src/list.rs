//! This module contains list enums for effects and their configs, in name and dispatch (instance
//! wrapper) form.

use serde::{Deserialize, Serialize};

/// This enum has a variant for each effect config, but only the names. If the `config-impls`
/// feature is enabled, then you can create an [`EffectConfigDispatchList`] by calling `.into()`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter, Serialize, Deserialize)]
pub enum EffectConfigNameList {
    /// See [`DebugOneByOneConfig`].
    DebugOneByOneConfig,

    /// See [`DebugBinaryIndexConfig`].
    DebugBinaryIndexConfig,

    /// See [`MovingPlaneConfig`].
    MovingPlaneConfig,

    /// See [`LavaLampConfig`].
    LavaLampConfig,
}

/// This enum has a variant for each effect, but only the names. If the `effect-impls` feature is
/// enabled, then you can call certain methods on this enum to get things like the like the
/// [`run`](Effect::run) method.
///
/// If an effect is not accessible via this enum, then it should not be used.
///
/// See `EffectDispatchList` for wrappers of instances of effects, or call `.into()` to read the
/// effect from its file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter, Serialize, Deserialize)]
pub enum EffectNameList {
    /// See [`DebugOneByOne`].
    DebugOneByOne,

    /// See [`DebugBinaryIndex`].
    DebugBinaryIndex,

    /// See [`MovingPlane`].
    MovingPlane,

    /// See [`LavaLamp`].
    LavaLamp,
}

cfg_if::cfg_if! {
    if #[cfg(feature = "config-impls")] {
        use crate::{
            effects::configs::*,
            traits::EffectConfig,
        };

        /// This enum has a variant to wrap an instance of every effect config. You can call most
        /// methods from the [`EffectConfig`] trait on a variant of this enum.
        #[derive(Clone, Debug, PartialEq, strum::EnumIter, Serialize, Deserialize)]
        pub enum EffectConfigDispatchList {
            /// See [`DebugOneByOneConfig`].
            DebugOneByOneConfig(DebugOneByOneConfig),

            /// See [`DebugBinaryIndexConfig`].
            DebugBinaryIndexConfig(DebugBinaryIndexConfig),

            /// See [`MovingPlaneConfig`].
            MovingPlaneConfig(MovingPlaneConfig),

            /// See [`LavaLampConfig`].
            LavaLampConfig(LavaLampConfig),
        }

        impl From<EffectConfigNameList> for EffectConfigDispatchList {
            fn from(value: EffectConfigNameList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectConfigNameList::DebugOneByOneConfig => {
                        EffectConfigDispatchList::DebugOneByOneConfig(DebugOneByOneConfig::from_file(
                            "DebugOneByOne",
                        ))
                    }
                    EffectConfigNameList::DebugBinaryIndexConfig => {
                        EffectConfigDispatchList::DebugBinaryIndexConfig(
                            DebugBinaryIndexConfig::from_file("DebugBinaryIndex"),
                        )
                    }
                    EffectConfigNameList::MovingPlaneConfig => {
                        EffectConfigDispatchList::MovingPlaneConfig(MovingPlaneConfig::from_file(
                            "MovingPlane",
                        ))
                    }
                    EffectConfigNameList::LavaLampConfig => {
                        EffectConfigDispatchList::LavaLampConfig(LavaLampConfig::from_file("LavaLamp"))
                    }
                }
            }
        }

        // TODO: Replace these with proc-macros (needs separate crate)
        impl EffectConfigDispatchList {
            pub fn render_options_gui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
                match self {
                    EffectConfigDispatchList::DebugOneByOneConfig(config) => {
                        config.render_options_gui(ctx, ui)
                    }
                    EffectConfigDispatchList::DebugBinaryIndexConfig(config) => {
                        config.render_options_gui(ctx, ui)
                    }
                    EffectConfigDispatchList::MovingPlaneConfig(config) => {
                        config.render_options_gui(ctx, ui)
                    }
                    EffectConfigDispatchList::LavaLampConfig(config) => {
                        config.render_options_gui(ctx, ui)
                    }
                }
            }

            pub fn save_to_file(&self, filename: &str)
            where
                Self: Sized + Serialize,
            {
                match self {
                    EffectConfigDispatchList::DebugOneByOneConfig(config) => {
                        crate::save_effect_config_to_file(filename, config)
                    }
                    EffectConfigDispatchList::DebugBinaryIndexConfig(config) => {
                        crate::save_effect_config_to_file(filename, config)
                    }
                    EffectConfigDispatchList::MovingPlaneConfig(config) => {
                        crate::save_effect_config_to_file(filename, config)
                    }
                    EffectConfigDispatchList::LavaLampConfig(config) => {
                        crate::save_effect_config_to_file(filename, config)
                    }
                }
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "effect-impls")] {
        use crate::{
            effects::effects::*,
            traits::Effect,
        };
        use ww_driver_trait::Driver;

        impl EffectNameList {
            // TODO: Replace this with a proc-macro (needs separate crate)
            pub fn effect_name(&self) -> &'static str
            where
                Self: Sized,
            {
                match self {
                    EffectNameList::DebugOneByOne => DebugOneByOne::effect_name(),
                    EffectNameList::DebugBinaryIndex => DebugBinaryIndex::effect_name(),
                    EffectNameList::MovingPlane => MovingPlane::effect_name(),
                    EffectNameList::LavaLamp => LavaLamp::effect_name(),
                }
            }

            pub fn config_from_file(&self) -> EffectConfigDispatchList {
                match self {
                    EffectNameList::DebugOneByOne => {
                        EffectConfigDispatchList::DebugOneByOneConfig(DebugOneByOne::config_from_file())
                    }
                    EffectNameList::DebugBinaryIndex => {
                        EffectConfigDispatchList::DebugBinaryIndexConfig(DebugBinaryIndex::config_from_file())
                    }
                    EffectNameList::MovingPlane => {
                        EffectConfigDispatchList::MovingPlaneConfig(MovingPlane::config_from_file())
                    }
                    EffectNameList::LavaLamp => {
                        EffectConfigDispatchList::LavaLampConfig(LavaLamp::config_from_file())
                    }
                }
            }
        }

        /// This enum has a variant to wrap an instance of every effect. You can call any method
        /// from the [`Effect`] trait on a variant of this enum.
        #[derive(Clone, Debug, PartialEq)]
        pub enum EffectDispatchList {
            /// See [`DebugOneByOne`].
            DebugOneByOne(DebugOneByOne),

            /// See [`DebugBinaryIndex`].
            DebugBinaryIndex(DebugBinaryIndex),

            /// See [`MovingPlane`].
            MovingPlane(MovingPlane),

            /// See [`LavaLamp`].
            LavaLamp(LavaLamp),
        }

        impl From<EffectNameList> for EffectDispatchList {
            fn from(value: EffectNameList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectNameList::DebugOneByOne => {
                        EffectDispatchList::DebugOneByOne(DebugOneByOne::from_file())
                    }
                    EffectNameList::DebugBinaryIndex => {
                        EffectDispatchList::DebugBinaryIndex(DebugBinaryIndex::from_file())
                    }
                    EffectNameList::MovingPlane => {
                        EffectDispatchList::MovingPlane(MovingPlane::from_file())
                    }
                    EffectNameList::LavaLamp => EffectDispatchList::LavaLamp(LavaLamp::from_file()),
                }
            }
        }

        impl From<EffectDispatchList> for EffectNameList {
            fn from(value: EffectDispatchList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectDispatchList::DebugOneByOne(_) => EffectNameList::DebugOneByOne,
                    EffectDispatchList::DebugBinaryIndex(_) => EffectNameList::DebugBinaryIndex,
                    EffectDispatchList::MovingPlane(_) => EffectNameList::MovingPlane,
                    EffectDispatchList::LavaLamp(_) => EffectNameList::LavaLamp,
                }
            }
        }

        impl From<&EffectDispatchList> for EffectNameList {
            fn from(value: &EffectDispatchList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectDispatchList::DebugOneByOne(_) => EffectNameList::DebugOneByOne,
                    EffectDispatchList::DebugBinaryIndex(_) => EffectNameList::DebugBinaryIndex,
                    EffectDispatchList::MovingPlane(_) => EffectNameList::MovingPlane,
                    EffectDispatchList::LavaLamp(_) => EffectNameList::LavaLamp,
                }
            }
        }

        impl From<EffectNameList> for EffectConfigNameList {
            fn from(value: EffectNameList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectNameList::DebugOneByOne => Self::DebugOneByOneConfig,
                    EffectNameList::DebugBinaryIndex => Self::DebugBinaryIndexConfig,
                    EffectNameList::MovingPlane => Self::MovingPlaneConfig,
                    EffectNameList::LavaLamp => Self::LavaLampConfig,
                }
            }
        }

        impl From<EffectConfigNameList> for EffectNameList {
            fn from(value: EffectConfigNameList) -> Self {
                // TODO: Replace this with a proc-macro (needs separate crate)
                match value {
                    EffectConfigNameList::DebugOneByOneConfig => Self::DebugOneByOne,
                    EffectConfigNameList::DebugBinaryIndexConfig => Self::DebugBinaryIndex,
                    EffectConfigNameList::MovingPlaneConfig => Self::MovingPlane,
                    EffectConfigNameList::LavaLampConfig => Self::LavaLamp,
                }
            }
        }

        // TODO: Replace these with proc-macros (needs separate crate)
        impl EffectDispatchList {
            pub fn effect_name(&self) -> &'static str
            where
                Self: Sized,
            {
                match self {
                    EffectDispatchList::DebugOneByOne(_) => DebugOneByOne::effect_name(),
                    EffectDispatchList::DebugBinaryIndex(_) => DebugBinaryIndex::effect_name(),
                    EffectDispatchList::MovingPlane(_) => MovingPlane::effect_name(),
                    EffectDispatchList::LavaLamp(_) => LavaLamp::effect_name(),
                }
            }

            pub async fn run(self, driver: &mut dyn Driver) {
                match self {
                    EffectDispatchList::DebugOneByOne(effect) => effect.run(driver).await,
                    EffectDispatchList::DebugBinaryIndex(effect) => effect.run(driver).await,
                    EffectDispatchList::MovingPlane(effect) => effect.run(driver).await,
                    EffectDispatchList::LavaLamp(effect) => effect.run(driver).await,
                }
            }

            pub fn save_to_file(&self) {
                match self {
                    EffectDispatchList::DebugOneByOne(effect) => effect.save_to_file(),
                    EffectDispatchList::DebugBinaryIndex(effect) => effect.save_to_file(),
                    EffectDispatchList::MovingPlane(effect) => effect.save_to_file(),
                    EffectDispatchList::LavaLamp(effect) => effect.save_to_file(),
                }
            }

            pub fn config_from_file(&self) -> EffectConfigDispatchList {
                match self {
                    EffectDispatchList::DebugOneByOne(_) => {
                        EffectConfigDispatchList::DebugOneByOneConfig(DebugOneByOne::config_from_file())
                    }
                    EffectDispatchList::DebugBinaryIndex(_) => {
                        EffectConfigDispatchList::DebugBinaryIndexConfig(DebugBinaryIndex::config_from_file())
                    }
                    EffectDispatchList::MovingPlane(_) => {
                        EffectConfigDispatchList::MovingPlaneConfig(MovingPlane::config_from_file())
                    }
                    EffectDispatchList::LavaLamp(_) => {
                        EffectConfigDispatchList::LavaLampConfig(LavaLamp::config_from_file())
                    }
                }
            }
        }
    }
}

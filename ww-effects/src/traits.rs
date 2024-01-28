//! This module contains the traits needed for effects and their associated configs to work.
//!
//! # Implementation
//!
//! Every individual effect must implement the [`Effect`] trait and have a config which is named
//! in a particular way and implements [`EffectConfig`].
//!
//! For example, imagine we have a new effect called `MyEffect`. The first step is deciding on a
//! category for the codebase. This is not user-facing, but is still important for organisation.
//! The categories are the submodules of [`crate::effects`].
//!
//! We should then create a file `ww-effects/src/effects/chosen_category/my_effect.rs`, which will
//! look something like this:
//! ```ignore
//! //! This module contains my effect.
//!
//! #[cfg(feature = "config-impls")]
//! pub use config::MyEffectConfig;
//!
//! #[cfg(feature = "effect-impls")]
//! pub use effect::MyEffect;
//!
//! use crate::effects::prelude::*;
//!
//! /// Contains the config for the [`MyEffect`] effect.
//! #[cfg(feature = "config-impls")]
//! mod config {
//!     use super::*;
//!
//!     /// The config for the [`MyEffect`] effect.
//!     #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
//!     pub struct MyEffectConfig {
//!         // Fields to describe the config of the effect
//!     }
//!
//!     impl Default for MyEffectConfig {
//!         fn default() -> Self {
//!             Self {
//!                 // Pick some sensible default config
//!             }
//!         }
//!     }
//!
//!     impl EffectConfig for MyEffectConfig {
//!         fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
//!             let mut config_changed = false;
//!
//!             // Put relevant GUI code here
//!
//!             config_changed
//!         }
//!     }
//! }
//!
//! /// Contains the [`MyEffect`] effect itself.
//! #[cfg(feature = "effect-impls")]
//! mod effect {
//!     use super::*;
//!
//!     /// Describe the effect.
//!     #[derive(Clone, Debug, PartialEq, BaseEffect)]
//!     pub struct MyEffect {
//!         // Fields to define the current state of the effect
//!         // The effect should NOT contain a field for its config. It should only use the config
//!         // passed into `next_frame`
//!     }
//!
//!     impl Effect for MyEffect {
//!         fn from_config(config: MyEffectConfig) -> Self {
//!             // Initialize the struct using its config
//!
//!             Self {}
//!         }
//!
//!         fn next_frame(&mut self, config: &MyEffectConfig) -> Option<(FrameType, Duration)> {
//!             // Generate the next frame of the effect. See the trait docs for what this should
//!             // return
//!         }
//!
//!         #[cfg(any(test, feature = "bench"))]
//!         fn loops_to_test() -> Option<NonZeroU16> {
//!             // This should be `None` if your effect is guaranteed to halt on its own,
//!             // otherwise it should be the number of loops to test or benchmark
//!             NonZeroU16::new(100)
//!         }
//!     }
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!     use crate::snapshot_effect;
//!
//!     #[test]
//!     fn my_effect_test() {
//!         snapshot_effect!(MyEffect);
//!     }
//! }
//! ```
//! You should look at the source code for the other effects to see how they do things.
//!
//! Once you've created your effect, the effect and its config should be publicly exported in
//! `ww-effects/src/effects/chosen_category/mod.rs` and `ww-effects/src/effects/mod.rs`. Follow the
//! examples of the other effects. Then add your effect to the
//! [`effect_proc_macros::generate_lists_and_impls`] list in `ww-effects/src/lib.rs`.

use egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use std::fs;

#[cfg(feature = "config-trait")]
use tracing_unwrap::ResultExt;

#[cfg(feature = "effect-trait")]
use std::time::Duration;
#[cfg(feature = "effect-trait")]
use ww_frame::FrameType;

/// Save the given effect config to its appropriate config file.
#[cfg(feature = "config-trait")]
pub fn save_effect_config_to_file<T>(filename: &str, config: &T)
where
    T: EffectConfig,
{
    let _ = fs::write(
        filename,
        ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default().struct_names(true))
            .expect_or_log("The effect config should be serializable"),
    );
}

/// Get the filename for the config of the given effect name.
#[cfg(feature = "config-impls")]
pub fn get_config_filename(effect_name: &str) -> String {
    use heck::ToSnakeCase;

    format!(
        "{}/config/{}.ron",
        std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined"),
        effect_name.to_snake_case()
    )
}

/// This module contains the [`Sealed`](self::private::Sealed) trait
#[cfg(any(feature = "config-trait", feature = "effect-trait"))]
pub(crate) mod private {
    #[cfg(doc)]
    use crate::{
        traits::{BaseEffect, BaseEffectConfig, Effect, EffectConfig},
        EffectNameList,
    };

    /// This trait restricts implementors of [`Effect`], [`BaseEffect`], [`BaseEffectConfig`], and
    /// [`EffectConfig`] to only be in this crate. This restriction is needed so that
    /// [`EffectNameList`] and friends have variants for all the effects.
    pub trait Sealed {}
}

/// A trait needed for all implemtors of [`EffectConfig`]. This trait should be derived with
/// [`effect_proc_macros::BaseEffectConfig`].
pub trait BaseEffectConfig:
    Clone + Default + PartialEq + Serialize + for<'de> Deserialize<'de> + private::Sealed
{
    /// Render the full options GUI, with a heading at the top and a "Reset to default" button at
    /// the bottom. The [derived
    /// implementation](../../effect_proc_macros/derive.BaseEffectConfig.html) will call [`<Self as
    /// EffectConfig>::render_options_gui`](EffectConfig::render_options_gui) in the middle.
    fn render_full_options_gui(&mut self, ctx: &Context, ui: &mut Ui) -> bool;
}

/// This trait is needed by all structs that want to act as configuration for effects.
#[cfg(feature = "config-trait")]
pub trait EffectConfig: BaseEffectConfig {
    /// Render the GUI to edit the config of this effect and return whether the config has changed.
    ///
    /// This method _SHOULD NOT_ include a heading or a _Reset to defaults_ button. These should be
    /// handled by [`BaseEffectConfig`], which can be derived.
    fn render_options_gui(&mut self, ctx: &Context, ui: &mut Ui) -> bool;

    /// Load the effect configuration from the config file, or use the default if the file is
    /// unavailable. Also save the default to the file for future editing.
    fn from_file(filename: &str) -> Self {
        let _ = fs::DirBuilder::new().recursive(true).create(format!(
            "{}/config",
            std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
        ));

        let write_and_return_default = || -> Self {
            let default = Self::default();
            save_effect_config_to_file(filename, &default);
            default
        };

        let Ok(text) = fs::read_to_string(filename) else {
            return write_and_return_default();
        };

        ron::from_str(&text).unwrap_or_else(|_| write_and_return_default())
    }

    /// Save the config to the given filename, which should be from the parent effect.
    fn save_to_file(&self, filename: &str) {
        save_effect_config_to_file(filename, self);
    }
}

/// A trait needed for all implemtors of [`Effect`]. This trait should be derived with
/// [`effect_proc_macros::BaseEffect`].
#[cfg(feature = "effect-trait")]
pub trait BaseEffect: Default + private::Sealed {
    /// The type of this effect's config.
    type Config: EffectConfig;

    /// The name of the effect, used for config files and GUI editting.
    fn effect_name() -> &'static str;
}

/// The trait implemented by all effects, which defines how to run them.
#[cfg(feature = "effect-trait")]
pub trait Effect: BaseEffect {
    /// The filename for the config file of this effect.
    fn config_filename() -> String {
        get_config_filename(Self::effect_name())
    }

    /// Return a copy of this effect's config, loaded from the file.
    fn config_from_file() -> Self::Config {
        Self::Config::from_file(&Self::config_filename())
    }

    /// Load the effect from the file.
    fn from_file() -> Self {
        Self::from_config(Self::config_from_file())
    }

    /// Create the effect from its config.
    fn from_config(config: <Self as BaseEffect>::Config) -> Self;

    /// Return the next frame of this effect, along with the duration that the server should wait
    /// before calling this function again.
    ///
    /// This function could return `None` to indicate that the effect is finished and the server
    /// should pause and restart it. This function should not handle looping the effect, and only
    /// run it once, unless it's a procedural aesthetic thing like
    /// [`LavaLamp`](../effects/aesthetic/struct.LavaLamp.html), then that should loop in this
    /// function.
    fn next_frame(
        &mut self,
        config: &<Self as BaseEffect>::Config,
    ) -> Option<(FrameType, Duration)>;

    /// How many loops should we run in test or benchmark builds?
    ///
    /// If this function returns `Some(number)`, then the test or benchmark will only call
    /// [`next_frame`] that many times. If this function returns `None`, then the test or benchmark
    /// will continue to call [`next_frame`] until it returns `None`.
    #[cfg(any(test, feature = "bench"))]
    fn loops_to_test() -> Option<std::num::NonZeroU16>;
}

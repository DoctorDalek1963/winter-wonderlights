//! This module contains the traits needed for effects and their associated configs to work.

use egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use std::fs;

/// Save the given effect config to its appropriate config file.
#[cfg(feature = "config-trait")]
pub fn save_effect_config_to_file<T>(filename: &str, config: &T)
where
    T: EffectConfig,
{
    let _ = fs::write(
        filename,
        ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default().struct_names(true))
            .expect("The effect config should be serializable"),
    );
}

/// Get the filename for the config of the given effect name.
#[cfg(feature = "config-impls")]
pub fn get_config_filename(effect_name: &str) -> String {
    use heck::ToSnakeCase;

    format!(
        "{}/config/{}.ron",
        env!("DATA_DIR"),
        effect_name.to_snake_case()
    )
}

/// This module contains the [`Sealed`](self::private::Sealed) trait
#[cfg(any(feature = "config-trait", feature = "effect-trait"))]
pub(crate) mod private {
    #[cfg(doc)]
    use super::{BaseEffect, Effect, EffectConfig};

    /// This trait restricts implementors of [`Effect`], [`BaseEffect`], and [`EffectConfig`] to only
    /// be in this crate. This restriction is needed so that
    /// [`EffectNameList`](../list/enum.EffectNameList.html) and friends have variants for all the
    /// effects.
    pub trait Sealed {}
}

/// This trait is needed by all structs that want to act as configuration for effects.
#[cfg(feature = "config-trait")]
pub trait EffectConfig:
    Clone + Default + PartialEq + Serialize + for<'de> Deserialize<'de> + private::Sealed
{
    /// Render the GUI to edit the config of this effect and return whether the config has changed.
    /// The default implementation returns false.
    ///
    /// If you implement this for an effect, the implementation should look something like the one
    /// below.
    ///
    /// ```ignore
    /// fn render_options_gui(&mut self, _ctx: &Context, ui: &mut Ui) {
    ///     ui.label(RichText::new("EffectName config").heading());
    ///
    ///     let mut config_changed = false;
    ///
    ///     // Implementation here...
    ///
    ///     if ui.button("Reset to defaults").clicked() {
    ///         *self = Self::default();
    ///         config_changed = true;
    ///     }
    ///
    ///     config_changed
    /// }
    /// ```
    #[allow(unused_variables)]
    fn render_options_gui(&mut self, _ctx: &Context, ui: &mut Ui) -> bool {
        false
    }

    /// Load the effect configuration from the config file, or use the default if the file is
    /// unavailable. Also save the default to the file for future editing.
    fn from_file(filename: &str) -> Self {
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .create(format!("{}/config", env!("DATA_DIR")));

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
    /// The name of the effect, used for config files and GUI editting.
    fn effect_name() -> &'static str;

    /// Save the config to a file.
    ///
    /// The implementation should call [`save_effect_config_to_file`] with
    /// `Self::config_filename()` and the internal config data.
    ///
    /// ```ignore
    /// fn save_to_file(&self) {
    ///     self.config.save_to_file(&Self::config_filename())
    /// }
    /// ```
    fn save_to_file(&self);

    /// Load the effect from a file.
    ///
    /// `Self::Config` will have a method [`from_file`](EffectConfig::from_file), so you can use
    /// that for the config. Any internal state should be initial state.
    ///
    /// The recommended implementation is shown below:
    ///
    /// ```ignore
    /// fn from_file() -> Self {
    ///     Self {
    ///         config: <Self as Effect>::Config::from_file(&Self::config_filename()),
    ///     }
    /// }
    /// ```
    fn from_file() -> Self;
}

/// The trait implemented by all effects, which defines how to run them.
#[cfg(feature = "effect-trait")]
pub trait Effect: BaseEffect {
    /// The type of this effect's config.
    type Config: EffectConfig;

    /// The filename for the config file of this effect.
    fn config_filename() -> String {
        get_config_filename(Self::effect_name())
    }

    /// Return a copy of this effect's config, loaded from the file.
    fn config_from_file() -> Self::Config {
        Self::Config::from_file(&Self::config_filename())
    }

    /// Run the effect with the given driver.
    ///
    /// This function should not handle looping the effect. That's handled by the driver code, so
    /// this function should just run the effect once.
    ///
    /// However, if the effect is a procedural aesthetic thing like
    /// [`LavaLamp`](../effects/aesthetic/struct.LavaLamp.html), then that should loop on its
    /// own.
    async fn run(self, driver: &mut dyn ww_driver_trait::Driver);
}

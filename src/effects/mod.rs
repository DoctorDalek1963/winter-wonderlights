//! This module provides lots of effects.

use crate::drivers::Driver;
use heck::ToSnakeCase;
use serde::{Deserialize, Serialize};
use std::fs;

mod debug;

pub use self::debug::{DebugBinaryIndex, DebugOneByOne};

/// Save the given effect config to its appropriate config file.
fn save_effect_config_to_file<T: Effect + Serialize>(effect: &T) {
    let _ = fs::write(
        T::config_filename(),
        ron::ser::to_string_pretty(effect, ron::ser::PrettyConfig::default().struct_names(true))
            .expect("The effect should be serializable"),
    );
}

/// The trait implemented by all effects.
///
/// The attributes of a struct implementing `Effect` are expected to represent the config of the
/// effect.
pub trait Effect {
    /// The name of the effect, used for config files and GUI editting.
    fn effect_name() -> &'static str
    where
        Self: Sized;

    /// The filename for the config file of this effect.
    fn config_filename() -> String
    where
        Self: Sized,
    {
        format!("config/{}.ron", Self::effect_name().to_snake_case())
    }

    /// The default configuration of the effect.
    fn default() -> Self
    where
        Self: Sized;

    /// Load the effect configuration from the config file, or use the default if the file is
    /// unavailable. Also save the default to the file for future editing.
    fn from_file() -> Self
    where
        Self: Sized + Serialize + for<'a> Deserialize<'a>,
    {
        let _ = fs::DirBuilder::new().recursive(true).create("config");
        let filename = format!("config/{}.ron", Self::effect_name().to_snake_case());

        let write_and_return_default = || -> Self {
            let default = Self::default();
            save_effect_config_to_file(&default);
            default
        };

        let Ok(text) = fs::read_to_string(filename) else {
            return write_and_return_default();
        };

        ron::from_str(&text).unwrap_or_else(|_| write_and_return_default())
    }

    /// Save the current effect config to the config file.
    fn save_effect_config_to_file(&self)
    where
        Self: Sized + Serialize,
    {
        save_effect_config_to_file(self);
    }

    /// Run the effect with the given driver.
    fn run(&mut self, driver: &mut dyn Driver);
}

//! This module provides lots of effects.

use crate::drivers::Driver;
use async_trait::async_trait;
use egui::{Context, Ui};
use heck::ToSnakeCase;
use serde::{Deserialize, Serialize};
use std::{fs, future::Future, pin::Pin};

mod debug;

pub use self::debug::{DebugBinaryIndex, DebugOneByOne};

/// An enum to list all the usable effects. If an effect is not accessible via this enum, then it
/// should not be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter)]
pub enum EffectList {
    /// See [`debug::DebugOneByOne`].
    DebugOneByOne,

    /// See [`debug::DebugBinaryIndex`].
    DebugBinaryIndex,
}

impl EffectList {
    /// Return a boxed async closure for the run method of the corresponding effect. Calling and
    /// awaiting the closure will run the effect.
    pub fn create_run_method(
        self,
    ) -> Box<dyn for<'a> Fn(&'a mut dyn Driver) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>>
    {
        /// A simple macro to generate a match statement to return a boxed closure for each effect.
        macro_rules! match_return_closures {
            ( $( $name:ident ),* ) => {
                match self {
                    $( EffectList::$name => Box::new(move |driver| $name::from_file().run(driver)), )*
                }
            };
        }

        match_return_closures!(DebugOneByOne, DebugBinaryIndex)
    }

    pub fn name(&self) -> &'static str {
        macro_rules! match_return_names {
            ( $( $name:ident ),* ) => {
                match *self {
                    $( EffectList::$name => $name::effect_name(), )*
                }
            };
        }

        match_return_names!(DebugOneByOne, DebugBinaryIndex)
    }

    pub fn config(&self) -> Box<dyn EffectConfig> {
        macro_rules! match_return_configs {
            ( $( $name:ident ),* ) => {
                match self {
                    $( EffectList::$name => Box::new($name::config()), )*
                }
            };
        }

        match_return_configs!(DebugOneByOne, DebugBinaryIndex)
    }
}

/// Save the given effect config to its appropriate config file.
pub fn save_effect_config_to_file<T>(filename: &str, config: &T)
where
    T: EffectConfig + Serialize,
{
    let _ = fs::write(
        filename,
        ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default().struct_names(true))
            .expect("The effect should be serializable"),
    );
}

/// This trait is needed by all structs that want to act as configuration for effects.
pub trait EffectConfig {
    /// Render the GUI to edit the config of this effect. The default implementation does nothing.
    ///
    /// If you implement this for an effect, the implementation should look something like this:
    ///
    /// ```
    /// # use egui::{Context, RichText, Ui};
    /// # use winter_wonderlights::effects::EffectConfig;
    /// # #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    /// # struct Dummy;
    /// # impl EffectConfig for Dummy {
    /// fn render_options_gui(&mut self, _ctx: &Context, ui: &mut Ui) {
    ///     ui.separator();
    ///     ui.label(RichText::new("EffectName config").heading());
    ///
    ///     // Implementation here...
    ///
    ///     if ui.button("Reset to defaults").clicked() {
    ///         *self = Self::default();
    ///     }
    /// }
    /// # }
    /// ```
    #[allow(unused_variables)]
    fn render_options_gui(&mut self, _ctx: &Context, ui: &mut Ui) {}

    /// Load the effect configuration from the config file, or use the default if the file is
    /// unavailable. Also save the default to the file for future editing.
    fn from_file(filename: &str) -> Self
    where
        Self: Default + Serialize + for<'a> Deserialize<'a>,
    {
        let _ = fs::DirBuilder::new().recursive(true).create("config");

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
    fn save_to_file(&self, filename: &str)
    where
        Self: Sized + Serialize,
    {
        save_effect_config_to_file(filename, self);
    }
}

/// The trait implemented by all effects, which primarily defines how to run them.
#[async_trait]
pub trait Effect: Default {
    /// The type of this effect's config.
    type Config: EffectConfig + Default + Serialize + for<'a> Deserialize<'a>;

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

    /// Return a copy of this effect's config.
    fn config() -> Self::Config {
        Self::Config::from_file(&Self::config_filename())
    }

    /// Run the effect with the given driver.
    async fn run(self, driver: &mut dyn Driver);

    /// Save the config to a file.
    ///
    /// The implementation should call [`save_effect_config_to_file`] with
    /// `Self::config_filename()` and the internal config data.
    ///
    /// ```
    /// # use winter_wonderlights::{drivers::Driver, effects::{Effect, EffectConfig}};
    /// # #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    /// # struct DummyConfig;
    /// # impl EffectConfig for DummyConfig {}
    /// # #[derive(Default)]
    /// # struct Dummy { config: DummyConfig };
    /// # #[async_trait::async_trait]
    /// # impl Effect for Dummy {
    /// # type Config = DummyConfig;
    /// # fn effect_name() -> &'static str { "Dummy" }
    /// # async fn run(self, driver: &mut dyn Driver) {}
    /// # fn from_file() -> Self { Self::default() }
    /// fn save_to_file(&self) {
    ///     self.config.save_to_file(&Self::config_filename())
    /// }
    /// # }
    fn save_to_file(&self);

    /// Load the effect from a file.
    ///
    /// `Self::Config` will have a method [`from_file`](EffectConfig::from_file), so you can use
    /// that for the config. Any internal state should be initial state.
    ///
    /// The recommended implementation is shown below:
    ///
    /// ```
    /// # use winter_wonderlights::{drivers::Driver,
    /// #     effects::{Effect, EffectConfig, save_effect_config_to_file}};
    /// # #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    /// # struct DummyConfig;
    /// # impl EffectConfig for DummyConfig {}
    /// # #[derive(Default)]
    /// # struct Dummy { config: DummyConfig };
    /// # #[async_trait::async_trait]
    /// # impl Effect for Dummy {
    /// # type Config = DummyConfig;
    /// # fn effect_name() -> &'static str { "Dummy" }
    /// # async fn run(self, driver: &mut dyn Driver) {}
    /// # fn save_to_file(&self) {}
    /// fn from_file() -> Self {
    ///     Self {
    ///         config: Self::Config::from_file(&Self::config_filename()),
    ///     }
    /// }
    /// # }
    /// ```
    fn from_file() -> Self;
}

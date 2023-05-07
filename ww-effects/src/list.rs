//! This module handles the [`EffectNameList`], which contains an entry for each possible effect.

use serde::{Deserialize, Serialize};

/// An enum to list all the usable effects. If an effect is not accessible via this enum, then it
/// should not be used.
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
    if #[cfg(feature = "impl")] {
        use crate::{
            aesthetic::LavaLamp,
            debug::{DebugBinaryIndex, DebugOneByOne},
            maths::MovingPlane,
            traits::{Effect, EffectConfig},
        };
        use std::{future::Future, pin::Pin};
        use ww_driver_trait::Driver;

        macro_rules! do_thing_on_effects {
            ( $thing:ident ) => {
                $thing!(DebugOneByOne, DebugBinaryIndex, MovingPlane, LavaLamp)
            };
        }

        // NOTE: For these macros to work, we need an effect in scope with the same name as its
        // corresponding entry in the enum.
        impl EffectNameList {
            /// Return a boxed async closure for the run method of the corresponding effect. Calling and
            /// awaiting the closure will run the effect.
            ///
            /// See [`Effect::run()`].
            pub fn create_run_method(
                self,
            ) -> Box<dyn for<'a> Fn(&'a mut dyn Driver) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>>
            {
                /// A simple macro to generate a match statement to return a boxed closure for each effect.
                macro_rules! match_return_closures {
                ( $( $name:ident ),* ) => {
                    match self {
                        $( EffectNameList::$name => Box::new(move |driver| $name::from_file().run(driver)), )*
                    }
                };
            }

                do_thing_on_effects!(match_return_closures)
            }

            /// Return the name of the selected effect.
            ///
            /// See [`Effect::effect_name()`].
            pub fn name(&self) -> &'static str {
                /// A simple macro to call `effect_name()` for the given effect.
                macro_rules! match_return_names {
                ( $( $name:ident ),* ) => {
                    match *self {
                        $( EffectNameList::$name => $name::effect_name(), )*
                    }
                };
            }

                do_thing_on_effects!(match_return_names)
            }

            /// Return a trait object of the config for the given effect, loaded from that effect's config
            /// file.
            ///
            /// See [`Effect::config_from_file()`].
            pub fn config_from_file(&self) -> Box<dyn EffectConfig> {
                /// A simple macro to call `config()` for the given effect.
                macro_rules! match_return_configs {
                ( $( $name:ident ),* ) => {
                    match self {
                        $( EffectNameList::$name => Box::new($name::config_from_file()), )*
                    }
                };
            }

                do_thing_on_effects!(match_return_configs)
            }
        }
    }
}

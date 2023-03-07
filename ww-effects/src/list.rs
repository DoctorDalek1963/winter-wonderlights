//! This module handles the [`EffectList`], which contains an entry for each possible effect.

use crate::{
    aesthetic::LavaLamp,
    debug::{DebugBinaryIndex, DebugOneByOne},
    maths::MovingPlane,
    traits::{Effect, EffectConfig},
};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
use ww_drivers::Driver;

/// An enum to list all the usable effects. If an effect is not accessible via this enum, then it
/// should not be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter, Serialize, Deserialize)]
pub enum EffectList {
    /// See [`DebugOneByOne`].
    DebugOneByOne,

    /// See [`DebugBinaryIndex`].
    DebugBinaryIndex,

    /// See [`MovingPlane`].
    MovingPlane,

    /// See [`LavaLamp`].
    LavaLamp,
}

macro_rules! do_thing_on_effects {
    ( $thing:ident ) => {
        $thing!(DebugOneByOne, DebugBinaryIndex, MovingPlane, LavaLamp)
    };
}

// NOTE: For these macros to work, we need an effect in scope with the same name as its
// corresponding entry in the enum.
impl EffectList {
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
                    $( EffectList::$name => Box::new(move |driver| $name::from_file().run(driver)), )*
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
                    $( EffectList::$name => $name::effect_name(), )*
                }
            };
        }

        do_thing_on_effects!(match_return_names)
    }

    /// Return a trait object of the config for the given effect.
    ///
    /// See [`Effect::config()`].
    pub fn config(&self) -> Box<dyn EffectConfig> {
        /// A simple macro to call `config()` for the given effect.
        macro_rules! match_return_configs {
            ( $( $name:ident ),* ) => {
                match self {
                    $( EffectList::$name => Box::new($name::config()), )*
                }
            };
        }

        do_thing_on_effects!(match_return_configs)
    }
}

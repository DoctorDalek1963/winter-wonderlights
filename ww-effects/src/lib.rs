//! This crate provides traits and implementations for various effects, as well as some utility
//! functions.

#![feature(async_fn_in_trait)]
#![feature(let_chains)]
#![feature(lint_reasons)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
#![cfg_attr(
    any(test, feature = "bench"),
    allow(
        unused_imports,
        reason = "std::time::Duration is imported for effects but unused in tests and benchmarks because of sleep!()"
    )
)]

pub mod list {
    //! This module contains list enums for effects and their configs, in name and dispatch (instance
    //! wrapper) form.

    effect_proc_macros::generate_lists_and_impls! {
        DebugOneByOne,
        DebugBinaryIndex,
        MovingPlane,
        SplitPlane,
        LavaLamp,
    }
}

pub use self::list::{EffectConfigNameList, EffectNameList};

#[cfg(feature = "config-impls")]
pub use self::list::EffectConfigDispatchList;

#[cfg(feature = "effect-impls")]
pub use self::list::EffectDispatchList;

#[cfg(any(feature = "effect-trait", feature = "config-trait"))]
pub mod traits;

#[cfg(feature = "config-trait")]
pub use self::traits::{save_effect_config_to_file, EffectConfig};

#[cfg(feature = "effect-trait")]
pub use self::traits::Effect;

#[cfg(any(feature = "config-impls", feature = "effect-impls"))]
pub mod effects;

cfg_if::cfg_if! {
    if #[cfg(test)] {
        use ww_driver_trait::Driver;
        use ww_frame::FrameType;

        /// A test driver that stores all the frames it receives so that they can be tested.
        pub struct TestDriver {
            pub lights_num: usize,
            pub data: Vec<FrameType>,
        }

        impl TestDriver {
            pub fn new(lights_num: usize) -> Self {
                Self {
                    lights_num,
                    data: vec![],
                }
            }
        }

        impl Driver for TestDriver {
            fn display_frame(&mut self, frame: FrameType) {
                self.data.push(frame);
            }

            fn get_lights_count(&self) -> usize {
                self.lights_num
            }
        }
    }
}

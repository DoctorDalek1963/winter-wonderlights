//! This crate provides traits and implementations for various effects, as well as some utility
//! functions. See the documentation for the [`traits`] module for details on how to implement your
//! own effect.

#![feature(let_chains)]
#![feature(lint_reasons)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

pub mod list {
    //! This module contains list enums for effects and their configs, in name and dispatch (instance
    //! wrapper) form.

    effect_proc_macros::generate_lists_and_impls! {
        DebugOneByOne,
        DebugBinaryIndex,
        MovingPlane,
        SplitPlane,
        LavaLamp,
        AiSnake,
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
            pub data: Vec<FrameType>,
        }

        impl TestDriver {
            pub fn new() -> Self {
                Self { data: vec![] }
            }

            pub fn test_effect<E: Effect>(&mut self) {
                let config = E::Config::default();
                let mut effect = E::from_config(config.clone());

                if let Some(number) = E::loops_to_test() {
                    self.data.reserve_exact(u16::from(number) as usize);

                    for i in 0..u16::from(number) {
                        if let Some((frame, _duration)) = effect.next_frame(&config) {
                            self.data.push(frame);
                        } else {
                            panic!(
                                "Effect {} said it would loop {number} times but terminated after only {i} loops",
                                E::effect_name()
                            );
                        }
                    }
                } else {
                    while let Some((frame, _duration)) = effect.next_frame(&config) {
                        self.data.push(frame);
                    }
                }
            }
        }

        impl Driver for TestDriver {
            unsafe fn init() -> Self {
                Self::new()
            }

            fn display_frame(&mut self, frame: FrameType) {
                self.data.push(frame);
            }
        }

        macro_rules! snapshot_effect {
            ($effect_type:ident) => {
                let mut driver = $crate::TestDriver::new();
                driver.test_effect::<$effect_type>();

                ::insta::assert_ron_snapshot!(driver.data);
            };
        }

        pub(crate) use snapshot_effect;
    }
}

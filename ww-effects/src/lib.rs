//! This crate provides traits and implementations for various effects, as well as some utility
//! functions.

// Duration is imported and unused for tests and benchmarks because of the sleep macro
#![cfg_attr(any(test, feature = "bench"), allow(unused_imports))]

pub mod list;

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

cfg_if::cfg_if! {
    if #[cfg(any(feature = "effect-impls", feature = "config-impls"))] {
        pub mod aesthetic;
        pub mod debug;
        pub mod maths;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "effect-impls")] {
        /// Asynchronously sleep for the specified duration and await it when running normally.
        ///
        /// The sleep call gets completely removed for test and bench builds.
        macro_rules! sleep {
            ( $dur:expr ) => {
                #[cfg(not(any(test, feature = "bench")))]
                ::tokio::time::sleep($dur).await
            };
        }

        pub(crate) use sleep;
        pub use self::{
            aesthetic::LavaLamp,
            debug::{DebugBinaryIndex, DebugOneByOne},
            maths::MovingPlane,
        };
    }
}

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

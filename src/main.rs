//! This binary crate just runs the program, currently just to test features.

use winter_wonderlights::{
    drivers::DebugDriver,
    effects::{DebugBinaryIndex, Effect},
};

fn main() {
    tracing_subscriber::fmt().init();

    let mut driver = DebugDriver { lights_num: 500 };
    DebugBinaryIndex {}.run(&mut driver);
}

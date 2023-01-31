//! This binary crate just runs the program, currently just to test features.

#[cfg(feature = "virtual-tree")]
use winter_wonderlights::drivers::VirtualTreeDriver;

#[cfg(not(feature = "virtual-tree"))]
use winter_wonderlights::drivers::DebugDriver;

use winter_wonderlights::effects::{DebugBinaryIndex, Effect};

fn main() {
    tracing_subscriber::fmt().init();

    #[cfg(feature = "virtual-tree")]
    let mut driver = VirtualTreeDriver {};

    #[cfg(not(feature = "virtual-tree"))]
    let mut driver = DebugDriver { lights_num: 500 };

    DebugBinaryIndex {}.run(&mut driver);
}

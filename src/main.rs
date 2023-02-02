//! This binary crate just runs the program, currently just to test features.

#[cfg(feature = "virtual-tree")]
use winter_wonderlights::drivers::run_effect_on_virtual_tree;

#[cfg(not(feature = "virtual-tree"))]
use winter_wonderlights::drivers::DebugDriver;

use winter_wonderlights::effects::DebugBinaryIndex;
#[cfg(not(feature = "virtual-tree"))]
use winter_wonderlights::effects::Effect;

fn main() {
    tracing_subscriber::fmt().init();

    #[cfg(feature = "virtual-tree")]
    run_effect_on_virtual_tree(Box::new(DebugBinaryIndex {}));

    #[cfg(not(feature = "virtual-tree"))]
    let mut driver = DebugDriver { lights_num: 500 };
    #[cfg(not(feature = "virtual-tree"))]
    DebugBinaryIndex {}.run(&mut driver);
}

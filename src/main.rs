//! This binary crate just runs the program, currently just to test features.

use cfg_if::cfg_if;
use winter_wonderlights::effects::{DebugBinaryIndex, Effect};

cfg_if! {
    if #[cfg(feature = "virtual-tree")] {
        use winter_wonderlights::drivers::run_effect_on_virtual_tree;
    }
    else {
        use winter_wonderlights::drivers::DebugDriver;
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    cfg_if! {
        if #[cfg(feature = "virtual-tree")] {
            run_effect_on_virtual_tree(Box::new(DebugBinaryIndex::from_file()));
        } else {
            let mut driver = DebugDriver { lights_num: 500 };
            DebugBinaryIndex::default().run(&mut driver);
        }
    }
}

//! This binary crate just runs the program, currently just to test features.

use cfg_if::cfg_if;
use winter_wonderlights::effects::EffectList;

cfg_if! {
    if #[cfg(feature = "virtual-tree")] {
        use winter_wonderlights::drivers::run_effect_on_virtual_tree;

        fn main() {
            run_effect_on_virtual_tree(EffectList::DebugBinaryIndex);
        }
    } else {
        use winter_wonderlights::drivers::DebugDriver;

        #[tokio::main(flavor = "current_thread")]
        async fn main() {
            let mut driver = DebugDriver { lights_num: 500 };
            EffectList::DebugBinaryIndex.create_run_method()(&mut driver).await;
        }
    }
}

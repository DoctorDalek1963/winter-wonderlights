//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "virtual-tree")] {
        use winter_wonderlights::drivers::run_virtual_tree;

        fn main() {
            tracing_subscriber::fmt::init();
            run_virtual_tree();
        }
    } else {
        use winter_wonderlights::{drivers::DebugDriver, effects::EffectList};

        #[tokio::main(flavor = "current_thread")]
        async fn main() {
            tracing_subscriber::fmt::init();
            let mut driver = DebugDriver { lights_num: 500 };
            EffectList::DebugBinaryIndex.create_run_method()(&mut driver).await;
        }
    }
}

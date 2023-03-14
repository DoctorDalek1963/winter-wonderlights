//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

use cfg_if::cfg_if;
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};

fn init_tracing() {
    let appender =
        tracing_appender::rolling::daily(concat!(env!("DATA_DIR"), "/logs"), "server.log");

    let subscriber = tracing_subscriber::registry()
        .with(
            Layer::new()
                .with_writer(appender)
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy(""),
                ),
        )
        .with(
            Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into())),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting the global default for tracing should be okay");
}

cfg_if! {
    if #[cfg(feature = "virtual-tree")] {
        use ww_driver_impl::run_virtual_tree;

        fn main() {
            init_tracing();
            run_virtual_tree();
        }
    } else {
        use ww_driver_impl::DebugDriver;
        use ww_effects::EffectList;

        #[tokio::main(flavor = "current_thread")]
        async fn main() {
            init_tracing();
            let mut driver = DebugDriver { lights_num: 500 };
            EffectList::DebugBinaryIndex.create_run_method()(&mut driver).await;
        }
    }
}

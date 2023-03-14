//! This crate implements a virtual tree driver to simulate effects with Bevy.

#![feature(never_type)]
#![feature(is_some_and)]

mod virtual_tree;

use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(
            Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::WARN.into())),
        ),
    )
    .expect("Setting the global default for tracing should be okay");

    self::virtual_tree::run_virtual_tree();
}

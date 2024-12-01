//! This crate implements a client interface for `Winter WonderLights`.

#![feature(let_chains)]
#![allow(
    clippy::future_not_send,
    reason = "the client primarily runs on Wasm, so multithreading async tasks is not possible"
)]

mod app;
mod camera;
mod controller;
mod generic_client;

use self::app::App;
use tracing_unwrap::ResultExt;

#[cfg(not(target_family = "wasm"))]
fn main() {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        follow_system_theme: true,
        ..Default::default()
    };

    eframe::run_native(
        "Winter WonderLights Scanner Client",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .expect_or_log("Unable to run native eframe app");
}

#[cfg(target_family = "wasm")]
fn main() {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            const MAX_TRACING_LEVEL: tracing::Level = tracing::Level::DEBUG;
        } else {
            const MAX_TRACING_LEVEL: tracing::Level = tracing::Level::INFO;
        }
    }

    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(MAX_TRACING_LEVEL)
            .build(),
    );

    let options = eframe::WebOptions {
        follow_system_theme: true,
        ..Default::default()
    };

    prokio::Runtime::default().spawn_pinned(move || async {
        eframe::web::WebRunner::new()
            .start(
                "main_canvas_id",
                options,
                Box::new(|cc| Box::new(App::new(cc))),
            )
            .await
            .expect_or_log("Unable to start WASM eframe app");
    });
}

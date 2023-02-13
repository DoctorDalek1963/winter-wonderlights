//! This module provides implementation for the virtual tree driver.

mod bevy_setup;
mod config;

use self::{
    bevy_setup::{add_tree_to_world, setup, LightIndex, TreeComponent},
    config::VirtualTreeConfig,
};
use crate::{
    drivers::Driver,
    effects::{EffectConfig, EffectList},
    frame::{FrameType, RGBArray},
    gift_coords::COORDS,
};
use bevy::{log::LogPlugin, prelude::*, DefaultPlugins};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::RichText;
use lazy_static::lazy_static;
use smooth_bevy_cameras::{controllers::orbit::OrbitCameraPlugin, LookTransformPlugin};
use std::{sync::RwLock, thread, time::Duration};
use strum::IntoEnumIterator;
use tokio::sync::broadcast;
use tracing::{debug, instrument, trace};

/// A global `RwLock` to record what the most recently sent frame is.
static CURRENT_FRAME: RwLock<FrameType> = RwLock::new(FrameType::Off);

/// The config for the virtual tree.
static mut VIRTUAL_TREE_CONFIG: VirtualTreeConfig = VirtualTreeConfig::default();

/// The trait object for the config of the current effect.
static mut EFFECT_CONFIG: Option<Box<dyn EffectConfig>> = None;

lazy_static! {
    /// The broadcast sender which lets you send messages to the calculation thread, which is
    /// running the effect itself.
    static ref SEND_MESSAGE_TO_THREAD: broadcast::Sender<ThreadMessage> = broadcast::channel(10).0;
}

/// Possible messages to send to the effect thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThreadMessage {
    /// Restart the effect rendering because a new effect has been selected.
    ///
    /// See [`VirtualTreeConfig.effect`].
    RestartNew,

    /// Restart the effect rendering with the current effect.
    RestartCurrent,
}

/// Set the global effect config. This method should be called after every time
/// [`VirtualTreeConfig.effect`] is updated.
fn set_global_effect_config() {
    unsafe { EFFECT_CONFIG = VIRTUAL_TREE_CONFIG.effect.map(|effect| effect.config()) };
}

/// Listen to messages on [`SEND_MESSAGE_TO_THREAD`] and run the effect in [`VIRTUAL_TREE_CONFIG`].
///
/// Intended to be run in a background thread.
fn listen_and_run_effect() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();

    thread::sleep(Duration::from_millis(1000));
    let mut driver = VirtualTreeDriver {};
    let mut rx = SEND_MESSAGE_TO_THREAD.subscribe();

    local.block_on(&runtime, async move {
        loop {
            // We always want to be selecting between two thing
            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = rx.recv() => {
                    match msg.expect("There should not be an error in receiving from the channel") {
                        ThreadMessage::RestartNew => {
                            set_global_effect_config();
                            continue;
                        }
                        ThreadMessage::RestartCurrent => continue,
                    };
                }

                // Then, we run the effect in a loop. Most of the effect time is awaiting
                // sleeps, and control gets yielded back to `select!` while that's happening,
                // so it can respond to messages quickly
                _ = async { loop {
                    if let Some(effect) = unsafe { VIRTUAL_TREE_CONFIG.effect } {
                        effect.create_run_method()(&mut driver).await;
                        driver.display_frame(FrameType::Off);

                        // Pause before looping the effect
                        tokio::time::sleep(
                            Duration::from_millis(unsafe { VIRTUAL_TREE_CONFIG.loop_pause_time })
                        ).await;
                    } else {
                        driver.display_frame(FrameType::Off);

                        // Don't send `FrameType::Off` constantly. `select!` takes control
                        // while we're awaiting anyway, so responding to a message will be fast
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }} => {}
            };
        }
    });
}

/// Run the virtual tree, using the saved or default config.
pub fn run_virtual_tree() -> ! {
    unsafe { VIRTUAL_TREE_CONFIG = VirtualTreeConfig::from_file() };
    set_global_effect_config();

    thread::spawn(listen_and_run_effect);

    // Create a new Bevy app with the default plugins (except logging, since we initialize that
    // ourselves) and the required systems
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "Winter WonderLights".to_string(),
                        ..default()
                    },
                    ..default()
                }),
        )
        .add_plugin(LookTransformPlugin)
        .add_plugin(OrbitCameraPlugin::default())
        .add_plugin(EguiPlugin)
        .add_startup_system(setup)
        .add_startup_system(add_tree_to_world)
        .add_system(update_lights)
        .add_system(render_gui)
        .add_system(show_hide_tree)
        .run();

    // Winit terminates the program after the event loop ends, so we should never get here. If we
    // do, then we want to terminate the program manually. We also want this function to return `!`
    std::process::exit(0);
}

/// A simple driver that uses a global [`RwLock`] to communicate with Bevy to render a virtual tree.
struct VirtualTreeDriver {}

impl Driver for VirtualTreeDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        info!(?frame);
        *CURRENT_FRAME.write().unwrap() = frame;
    }

    fn get_lights_count(&self) -> usize {
        COORDS.coords().len()
    }
}

/// Update the lights by reading from the [`RwLock`] and setting the colours of all the lights.
#[instrument(skip_all)]
fn update_lights(
    mut materials: ResMut<Assets<StandardMaterial>>,
    parent_query: Query<(&Handle<StandardMaterial>, &LightIndex, &Children)>,
    mut child_query: Query<&mut PointLight>,
) {
    let Ok(frame) = CURRENT_FRAME.try_read() else {
        return;
    };
    let frame = frame.clone();
    debug!("Updating lights, frame = {frame:?}");

    let mut render_raw_data = |vec: Vec<RGBArray>| {
        for (handle, idx, children) in parent_query.iter() {
            // Set emissive colour
            let mut mat = materials.get(handle).unwrap().clone();
            trace!(?idx, "Before, color = {:?}", mat.emissive);

            let [r, g, b] = vec[idx.0];
            let new_colour = Color::rgb_u8(r, g, b).as_rgba_linear();

            mat.emissive = new_colour;
            trace!(?idx, "After, color = {:?}", mat.emissive);
            let _ = materials.set(handle, mat);

            for &child in children.iter() {
                // Set colour of light
                let mut point_light = child_query.get_mut(child).unwrap();
                point_light.color = new_colour;
            }
        }
    };

    match frame {
        FrameType::Off => render_raw_data(vec![[0, 0, 0]; COORDS.lights_num()]),
        FrameType::RawData(vec) => render_raw_data(vec),
        FrameType::Frame3D(frame) => render_raw_data(frame.to_raw_data()),
    }
}

/// Render the configuration GUI, which has config options for this virtual tree, as well as a
/// section for the config of the current effect.
fn render_gui(mut ctx: ResMut<EguiContext>) {
    let ctx = ctx.ctx_mut();
    egui::Window::new("Config").show(ctx, |ui| {
        ui.label(RichText::new("Virtual tree config").heading());
        let mut config_changed = false;

        config_changed |= ui
            .add(
                egui::Slider::new(
                    unsafe { &mut VIRTUAL_TREE_CONFIG.loop_pause_time },
                    0..=3000,
                )
                .suffix("ms")
                .text("Loop pause time"),
            )
            .changed();

        config_changed |= ui
            .checkbox(
                unsafe { &mut VIRTUAL_TREE_CONFIG.is_tree_enabled },
                "Show tree",
            )
            .changed();

        let new_effect_selected = egui::ComboBox::from_label("Current effect")
            .selected_text(unsafe {
                VIRTUAL_TREE_CONFIG
                    .effect
                    .map_or("None", |effect| effect.name())
            })
            .show_ui(ui, |ui| {
                let selected_none = ui
                    .selectable_value(unsafe { &mut VIRTUAL_TREE_CONFIG.effect }, None, "None")
                    .clicked();

                // Iterate over all the effects and see if a new effect has been clicked
                let selected_new_effect = EffectList::iter().any(|effect| {
                    // We remember which value was initially selected and whether this value is a
                    // new one
                    let different = Some(effect) != unsafe { VIRTUAL_TREE_CONFIG.effect };
                    let resp = ui.selectable_value(
                        unsafe { &mut VIRTUAL_TREE_CONFIG.effect },
                        Some(effect),
                        effect.name(),
                    );

                    // If the value is different from the old and has been clicked, then we care
                    resp.clicked() && different
                });

                selected_new_effect || selected_none
            })
            .inner
            .is_some_and(|x| x);

        if new_effect_selected {
            config_changed = true;
            SEND_MESSAGE_TO_THREAD
                .send(ThreadMessage::RestartNew)
                .expect("There should not be an error sending the restart message");
        }

        if config_changed {
            unsafe {
                VIRTUAL_TREE_CONFIG.save_to_file();
            }
        }

        if ui.button("Restart current effect").clicked() {
            SEND_MESSAGE_TO_THREAD
                .send(ThreadMessage::RestartCurrent)
                .expect("There should not be an error sending the restart message");
        }

        if let Some(config) = unsafe { &mut EFFECT_CONFIG } {
            config.render_options_gui(ctx, ui);
        }
    });
}

/// Show or hide the tree depending on the value of [`VirtualTreeConfig::is_tree_enabled`].
fn show_hide_tree(mut query: Query<&mut Visibility, With<TreeComponent>>) {
    for mut entity in query.iter_mut() {
        entity.is_visible = unsafe { VIRTUAL_TREE_CONFIG.is_tree_enabled };
    }
}

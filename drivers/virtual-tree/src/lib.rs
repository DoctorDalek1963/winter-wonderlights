//! This module provides implementation for the virtual tree driver.

mod bevy_setup;

use self::bevy_setup::{add_tree_to_world, setup, LightIndex};
use bevy::{log::LogPlugin, prelude::*, DefaultPlugins};
use smooth_bevy_cameras::{controllers::orbit::OrbitCameraPlugin, LookTransformPlugin};
use std::sync::RwLock;
use tracing::{debug, instrument, trace};
use ww_driver_trait::Driver;
use ww_frame::{FrameType, RGBArray};
use ww_gift_coords::COORDS;

/// A global `RwLock` to record what the most recently sent frame is.
static CURRENT_FRAME: RwLock<FrameType> = RwLock::new(FrameType::Off);

/// Run the virtual tree with Bevy.
#[instrument]
pub fn run_virtual_tree() {
    // Create a new Bevy app with the default plugins (except logging, since that's handled by the
    // server) and the required systems
    info!("Starting bevy app");
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "Winter WonderLights Virtual Tree".to_string(),
                        ..default()
                    },
                    ..default()
                }),
        )
        .add_plugin(LookTransformPlugin)
        .add_plugin(OrbitCameraPlugin::default())
        .add_startup_system(setup)
        .add_startup_system(add_tree_to_world)
        .add_system(update_lights)
        .run();

    // Winit terminates the program after the event loop ends, so we should never get here. If we
    // do, then we want to terminate the program manually. We also want this function to return `!`
    tracing::error!(concat!(
        "Winit should terminate the program when the eventloop ends, but it hasn't. ",
        "Now terminating the program."
    ));
    std::process::exit(0);
}

/// A simple driver that uses a global [`RwLock`] to communicate with Bevy to render a virtual tree.
pub struct VirtualTreeDriver {}

impl VirtualTreeDriver {
    pub fn init() -> Self {
        Self {}
    }
}

impl Driver for VirtualTreeDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        debug!(?frame);
        *CURRENT_FRAME.write().unwrap() = frame;
    }

    fn get_lights_count(&self) -> usize {
        COORDS.lights_num()
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
    trace!(?frame);

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

//! This crate provides a binary which can be run by the `virtual-tree` driver to do the actual
//! graphical simulation.
//!
//! The driver runs this binary crate and uses IPC to update the [`CURRENT_FRAME`]. This separation
//! is necessary because Bevy needs `winit`, which needs to be run on the main thread, so we use
//! the main thread of a different application.

mod bevy_setup;

use self::bevy_setup::{add_tree_to_world, setup, LightIndex};
use bevy::{log::LogPlugin, prelude::*, DefaultPlugins};
use interprocess_docfix::local_socket::LocalSocketStream;
use smooth_bevy_cameras::{controllers::orbit::OrbitCameraPlugin, LookTransformPlugin};
use std::{
    env,
    io::{self, Read},
    process,
    sync::RwLock,
    thread,
};
use tracing::{debug, error, instrument, trace, warn, Level};
use tracing_unwrap::{OptionExt, ResultExt};
use virtual_tree_shared::Message;
use ww_frame::{FrameType, RGBArray};
use ww_gift_coords::COORDS;

/// A global `RwLock` to record what the most recently sent frame is.
static CURRENT_FRAME: RwLock<(FrameType, u8)> = RwLock::new((FrameType::Off, 100));

/// Start the runner, taking a path to a local socket as the first command line argument.
fn main() {
    tracing_subscriber::fmt().with_max_level(Level::WARN).init();

    let socket_path = env::args()
        .nth(1)
        .expect_or_log("We need a socket path as the first argument");
    debug!(?socket_path);

    thread::Builder::new()
        .name("listen-to-virtual-tree-socket".to_string())
        .spawn(move || listen_to_socket(&socket_path))
        .unwrap_or_log();

    run_virtual_tree();
}

/// Listen to the given socket and update [`CURRENT_FRAME`] when the socket tells us to update the
/// frame.
#[instrument]
fn listen_to_socket(socket_path: &str) {
    let mut conn = LocalSocketStream::connect(socket_path)
        .expect_or_log(&format!("Unable to connect to socket at {socket_path:?}"));
    let mut buf = [0_u8; 5180]; // 5kB

    loop {
        let idx = conn
            .read(&mut buf)
            .expect_or_log("We should be able to read from the socket connection");
        let message: Message = match bincode::deserialize(&buf[..idx]) {
            Ok(msg) => msg,
            Err(e) => match *e {
                bincode::ErrorKind::Io(e) if e.kind() == io::ErrorKind::UnexpectedEof => continue,
                e => {
                    error!(?e, "Unexpected error");
                    continue;
                }
            },
        };
        trace!(?message, "Deserialized message");

        match message {
            Message::UpdateFrame(frame, max_brightness) => {
                *CURRENT_FRAME.write().unwrap_or_log() = (frame, max_brightness);
            }
            Message::Shutdown => process::exit(0),
        };
    }
}

/// Run the virtual tree with Bevy.
#[instrument]
fn run_virtual_tree() {
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
    warn!(concat!(
        "Winit should terminate the program when the eventloop ends, but it hasn't. ",
        "Now terminating the program."
    ));
    process::exit(255);
}

/// Update the lights by reading from the [`RwLock`] and setting the colours of all the lights.
#[instrument(skip_all)]
fn update_lights(
    mut materials: ResMut<Assets<StandardMaterial>>,
    parent_query: Query<(&Handle<StandardMaterial>, &LightIndex, &Children)>,
    mut child_query: Query<&mut PointLight>,
) {
    let Ok(rw_lock) = CURRENT_FRAME.try_read() else {
        return;
    };
    let (frame, max_brightness) = rw_lock.clone();
    trace!(?frame, ?max_brightness);

    let brightness_factor = max_brightness as f32 / 100.;
    debug_assert!(
        (0.0..=1.0).contains(&brightness_factor),
        "brightness_factor must be between 0. and 1."
    );

    let mut render_raw_data = |vec: Vec<RGBArray>| {
        for (handle, idx, children) in parent_query.iter() {
            // Set emissive colour
            let mut mat = materials.get(handle).unwrap_or_log().clone();
            trace!(?idx, "Before, color = {:?}", mat.emissive);

            let [r, g, b] = vec[idx.0];
            let [hue, saturation, lightness, alpha] = Color::rgb_u8(r, g, b).as_hsla_f32();
            let new_colour = Color::hsla(hue, saturation, lightness * brightness_factor, alpha);

            mat.emissive = new_colour;
            trace!(?idx, "After, color = {:?}", mat.emissive);
            let _ = materials.set(handle, mat);

            for &child in children {
                // Set colour of light
                let mut point_light = child_query.get_mut(child).unwrap_or_log();
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

//! This module provides implementation for the virtual tree driver.

use crate::{drivers::Driver, effects::Effect, frame::FrameType, gift_coords::GIFTCoords};
use bevy::{core_pipeline::bloom::BloomSettings, log::LogPlugin, prelude::*, DefaultPlugins};
use lazy_static::lazy_static;
use smooth_bevy_cameras::{
    controllers::orbit::{OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin},
    LookTransformPlugin,
};
use std::{sync::RwLock, thread, time::Duration};
use tracing::{debug, instrument};

/// A global `RwLock` to record what the most recently sent frame is.
static FRAME_RW_LOCK: RwLock<FrameType> = RwLock::new(FrameType::Off);

lazy_static! {
    /// The GIFTCoords loaded from `coords.gift`.
    static ref COORDS: GIFTCoords =
        GIFTCoords::from_file("coords.gift").expect("We need the coordinates to build the tree");
}

/// Run the given effect on the virtual tree.
///
/// This function is necessary because the [`VirtualTreeDriver`] is a bit different because it uses
/// Bevy to render everything. Bevy uses Winit for its windows, but Winit needs to run on the main
/// thread. This function just spawns a background thread to run the effect itself and then runs a
/// Bevy app on the main thread.
pub fn run_effect_on_virtual_tree(mut effect: Box<dyn Effect + Send>) -> ! {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));

        loop {
            let mut driver = VirtualTreeDriver {};
            effect.run(&mut driver);
            driver.display_frame(FrameType::Off);

            // Pause for 1.5 seconds before looping the effect
            thread::sleep(Duration::from_millis(1500));
        }
    });

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
        .add_startup_system(setup)
        .add_system(update_lights)
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
        *FRAME_RW_LOCK.write().unwrap() = frame;
    }

    fn get_lights_count(&self) -> usize {
        COORDS.coords().len()
    }
}

/// A simple Bevy component to record the index of this light along the chain of lights.
#[derive(Component, Clone, Copy, Debug)]
struct LightIndex(usize);

/// Setup the Bevy world with a camera, plane, and lights.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Hold LControl to orbit the camera
    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                ..default()
            },
            BloomSettings {
                intensity: 1.4,
                threshold: 0.6,
                ..default()
            },
        ))
        .insert(OrbitCameraBundle::new(
            OrbitCameraController {
                mouse_rotate_sensitivity: Vec2::splat(0.25),
                smoothing_weight: 0.1,
                ..default()
            },
            Vec3::new(5., 2.5, 5.),
            Vec3::new(0., COORDS.max_z() as f32 / 2., 0.),
            Vec3::Y,
        ));

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 1000. })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.7, 0.7, 0.7),
            perceptual_roughness: 0.08,
            ..default()
        }),
        transform: Transform::from_xyz(0., -0.1, 0.),
        ..default()
    });

    // One sphere mesh for the lights
    let mesh = meshes.add(Mesh::from(shape::UVSphere {
        sectors: 64,
        stacks: 32,
        radius: 0.015,
    }));

    // All the lights
    for (index, &(x, z, y)) in COORDS.coords().iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgba(0.1, 0.1, 0.1, 0.5),
                    unlit: false,
                    emissive: Color::rgb_linear(0.1, 0.1, 0.1),
                    ..default()
                }),
                transform: Transform::from_xyz(x as f32, y as f32, z as f32),
                ..default()
            },
            LightIndex(index),
        ));
    }
}

/// Update the lights by reading from the [`RwLock`] and setting the colours of all the lights.
#[instrument(skip_all)]
fn update_lights(
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&Handle<StandardMaterial>, &LightIndex)>,
) {
    let Ok(frame) = FRAME_RW_LOCK.try_read() else {
        return;
    };
    let frame = frame.clone();
    debug!("Updating lights, frame = {frame:?}");

    match frame {
        FrameType::Off => {
            for (handle, idx) in query.iter() {
                let mut mat = materials.get(&handle).unwrap().clone();
                debug!(?idx, "Before, color = {:?}", mat.emissive);

                mat.emissive = Color::rgb(0., 0., 0.).as_rgba_linear();
                debug!(?idx, "After, color = {:?}", mat.emissive);
                let _ = materials.set(handle, mat);
            }
        }
        FrameType::RawData(vec) => {
            for (handle, idx) in query.iter() {
                let mut mat = materials.get(&handle).unwrap().clone();
                debug!(?idx, "Before, color = {:?}", mat.emissive);

                let (r, g, b) = vec[idx.0];
                mat.emissive = Color::rgb_u8(r, g, b).as_rgba_linear();
                debug!(?idx, "After, color = {:?}", mat.emissive);
                let _ = materials.set(handle, mat);
            }
        }
        FrameType::Frame3D(_) => unimplemented!(),
    }
}

//! This module provides implementation for the virtual tree driver.

use crate::{drivers::Driver, effects::Effect, frame::FrameType};
use bevy::{log::LogPlugin, prelude::*, DefaultPlugins};
use std::{sync::RwLock, thread, time::Duration};
use tracing::{debug, instrument};

/// This is a temporary constant until coordinates are implemented.
const LIGHTS_NUM: usize = 32;

/// A global RwLock to record what the most recently sent frame is.
static FRAME_RW_LOCK: RwLock<FrameType> = RwLock::new(FrameType::Off);

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
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_startup_system(setup)
        .add_system(update_lights)
        .run();

    // Winit terminates the program after the event loop ends, so we should never get here. If we
    // do, then we want to terminate the program manually. We also want this function to return `!`
    std::process::exit(0);
}

/// A simple driver that uses a global RwLock to communicate with Bevy to render a virtual tree.
struct VirtualTreeDriver {}

impl Driver for VirtualTreeDriver {
    #[instrument(skip_all)]
    fn display_frame(&mut self, frame: FrameType) {
        debug!(?frame);
        *FRAME_RW_LOCK.write().unwrap() = frame;
    }

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(LIGHTS_NUM as f32 / 2., 2.5, 30.).looking_at(
            Vec3 {
                x: LIGHTS_NUM as f32 / 2.,
                y: 0.,
                z: 0.,
            },
            Vec3::Y,
        ),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: LIGHTS_NUM as f32 * 10.0,
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.2, 0.2, 0.2),
            perceptual_roughness: 0.08,
            ..default()
        }),
        transform: Transform::from_xyz(0., -0.1, 0.),
        ..default()
    });

    let mesh = meshes.add(Mesh::from(shape::UVSphere {
        sectors: 128,
        stacks: 64,
        radius: 0.1,
    }));

    for index in 0..LIGHTS_NUM {
        commands
            .spawn(PbrBundle {
                mesh: mesh.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgba(0.1, 0.1, 0.1, 0.5),
                    unlit: true,
                    ..default()
                }),
                transform: Transform::from_xyz(index as f32, 0., 0.),
                ..default()
            })
            .with_children(|children| {
                children.spawn((
                    PointLightBundle {
                        point_light: PointLight {
                            color: Color::rgb(0.2, 0.2, 1.0),
                            intensity: 1500.0,
                            radius: 0.2,
                            range: 2.,
                            ..default()
                        },
                        ..default()
                    },
                    LightIndex(index),
                ));
            });
    }
}

/// Update the lights by reading from the RwLock and setting the colours of all the lights.
#[instrument(skip_all)]
fn update_lights(mut query: Query<(&mut PointLight, &LightIndex)>) {
    let Ok(frame) = FRAME_RW_LOCK.try_read() else {
        return;
    };
    let frame = frame.clone();
    debug!(?frame, ?query, "Updating lights");

    match frame {
        FrameType::Off => {
            for (mut light, _idx) in query.iter_mut() {
                light.color = Color::rgb(0., 0., 0.);
            }
        }
        FrameType::RawData(vec) => {
            for (mut light, idx) in query.iter_mut() {
                debug!(?light, ?idx, "Before");
                let (r, g, b) = vec[idx.0];
                light.color = Color::rgb_u8(r, g, b);
                debug!(?light, ?idx, "After");
            }
        }
        FrameType::Frame3D(_) => unimplemented!(),
    }
}

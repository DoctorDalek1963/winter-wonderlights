//! This module handles things to setup bevy for the virtual tree.

use crate::gift_coords::COORDS;
use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use smooth_bevy_cameras::controllers::orbit::{OrbitCameraBundle, OrbitCameraController};

/// A simple Bevy component to record the index of this light along the chain of lights.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct LightIndex(pub(super) usize);

/// Setup the Bevy world with a camera, plane, and lights.
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // TODO: Generate mesh for tree, maybe hash GIFT coords to cache tree mesh

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
            base_color: Color::rgb(0.9, 0.9, 0.9),
            perceptual_roughness: 0.8,
            ..default()
        }),
        transform: Transform::from_xyz(0., -0.3, 0.),
        ..default()
    });

    // Tree
    commands.spawn(PbrBundle {
        mesh: meshes.add(generate_tree_mesh()),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb_u8(39, 13, 13),
            perceptual_roughness: 0.8,
            ..default()
        }),
        transform: Transform::from_xyz(0., COORDS.max_z() as f32 / 2. - 0.2, 0.),
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
        commands
            .spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgba(0.1, 0.1, 0.1, 0.5),
                        unlit: false,
                        emissive: Color::rgb_linear(0.1, 0.1, 0.1),
                        perceptual_roughness: 0.8,
                        ..default()
                    }),
                    transform: Transform::from_xyz(x as f32, y as f32, z as f32),
                    ..default()
                },
                LightIndex(index),
            ))
            .with_children(|builder| {
                builder.spawn(PointLightBundle {
                    point_light: PointLight {
                        color: Color::rgb(0., 0., 0.),
                        intensity: 1.5,
                        range: 0.8,
                        shadows_enabled: false,
                        ..default()
                    },
                    ..default()
                });
            });
    }
}

/// Generate the mesh to use for the Christmas tree itself
fn generate_tree_mesh() -> Mesh {
    //let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    shape::Capsule {
        radius: 0.06,
        rings: 100,
        depth: COORDS.max_z() as f32 * 0.88,
        ..default()
    }
    .into()
}

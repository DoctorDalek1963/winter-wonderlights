//! This module handles things to setup bevy for the virtual tree.

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use rand::{thread_rng, Rng};
use smooth_bevy_cameras::controllers::orbit::{OrbitCameraBundle, OrbitCameraController};
use std::f32::consts::PI;
use ww_gift_coords::COORDS;

/// A simple Bevy component to record the index of this light along the chain of lights.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct LightIndex(pub(super) usize);

/// Setup the Bevy world with a camera, plane, and lights.
pub(super) fn setup(
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
            Vec3::new(0., COORDS.max_z() / 2., 0.),
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

    // One sphere mesh for the lights
    let mesh = meshes.add(Mesh::from(shape::UVSphere {
        sectors: 64,
        stacks: 32,
        radius: 0.012,
    }));

    // All the lights
    debug!("Adding lights to tree");
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
                    transform: Transform::from_xyz(x, y, z),
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
    debug!("Finished adding lights to tree");
}

/// Add the Christmas tree to the world.
pub(super) fn add_tree_to_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Trunk
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Capsule {
            radius: 0.06,
            rings: 100,
            depth: COORDS.max_z() * 0.88,
            ..default()
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb_u8(39, 13, 13),
            perceptual_roughness: 0.8,
            ..default()
        }),
        transform: Transform::from_xyz(0., COORDS.max_z() / 2. - 0.2, 0.),
        ..default()
    });

    // Leaves
    let initial_y: f32 = 0.3;
    let max_y: f32 = COORDS.max_z() - 0.3;
    let mut y = initial_y;

    let mut rng = thread_rng();
    let leaf_material = materials.add(StandardMaterial {
        base_color: Color::rgb_u8(12, 96, 29),
        perceptual_roughness: 0.85,
        ..default()
    });

    while y < max_y {
        let scale = (1. - (y - initial_y) / (max_y - initial_y)).clamp(0.1, 1.);

        // We want a random number of branches equally spaced around the trunk. Giving them a
        // random starting offset makes them less predictable
        let num_branches_proportion = 360 / rng.gen_range(10..=16);
        for theta in (0..360)
            .skip(rng.gen_range(0..=num_branches_proportion))
            .step_by(num_branches_proportion)
        {
            // Create a capsule shape connecting the core of the trunk to a point away from the
            // trunk at `theta` degrees around

            let theta_rad = theta as f32 / 180. * PI;
            let point = (theta_rad.sin() * scale, y, theta_rad.cos() * scale);
            let (mesh, transform) =
                create_tree_branch((0., y, 0.), point, (scale * 0.03).max(0.015), &mut rng);

            commands.spawn(PbrBundle {
                mesh: meshes.add(mesh),
                transform,
                material: leaf_material.clone(),
                ..default()
            });
        }

        y += 0.15;
    }
}

/// Create a tree branch connecting the two given points with the given radius, using the given RNG
/// to add variety to the branch rotations.
fn create_tree_branch(
    p: (f32, f32, f32),
    q: (f32, f32, f32),
    radius: f32,
    rng: &mut impl Rng,
) -> (Mesh, Transform) {
    let (px, py, pz) = p;
    let (qx, qy, qz) = q;

    let length = {
        let dx = (px - qx).abs();
        let dy = (py - qy).abs();
        let dz = (pz - qz).abs();

        f32::sqrt(dx * dx + dy * dy + dz * dz)
    };
    let midpoint = ((px + qx) / 2., (py + qy) / 2., (pz + qz) / 2.);

    let mesh = shape::Capsule {
        radius,
        rings: 50,
        depth: length,
        ..default()
    }
    .into();

    let p_to_q = Vec3 {
        x: px - qx + rng.gen_range(-0.1..=0.1),
        y: py - qy + rng.gen_range(-0.1..=0.1),
        z: pz - qz + rng.gen_range(-0.1..=0.1),
    };
    let transform = Transform::from_xyz(midpoint.0, midpoint.1, midpoint.2)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, p_to_q.normalize()));

    (mesh, transform)
}

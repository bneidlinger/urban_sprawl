//! Window lights for buildings that illuminate at night.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::day_night::TimeOfDay;

pub struct WindowLightsPlugin;

impl Plugin for WindowLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowLightConfig>()
            .add_systems(Update, spawn_window_lights.run_if(should_spawn_windows))
            .add_systems(Update, update_window_brightness);
    }
}

fn should_spawn_windows(
    spawned: Res<BuildingsSpawned>,
    window_query: Query<&WindowLight>,
) -> bool {
    spawned.0 && window_query.is_empty()
}

#[derive(Component)]
pub struct WindowLight {
    /// Whether this window is "occupied" (will light up at night)
    pub occupied: bool,
    /// Base emissive intensity when lit
    pub intensity: f32,
    /// Light color for this window
    pub color: LinearRgba,
}

#[derive(Resource)]
pub struct WindowLightConfig {
    pub window_width: f32,
    pub window_height: f32,
    pub floor_height: f32,
    pub windows_per_floor: usize,
    pub occupancy_rate: f32,
    pub seed: u64,
}

impl Default for WindowLightConfig {
    fn default() -> Self {
        Self {
            window_width: 1.2,
            window_height: 1.5,
            floor_height: 3.5,
            windows_per_floor: 4,
            occupancy_rate: 0.6, // 60% of windows are "occupied"
            seed: 77777,
        }
    }
}

fn spawn_window_lights(
    mut commands: Commands,
    config: Res<WindowLightConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning window lights...");

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Window mesh (small flat quad)
    let window_mesh = meshes.add(Cuboid::new(
        config.window_width,
        config.window_height,
        0.1,
    ));

    // Window materials - warm interior light colors
    let window_colors = [
        Color::srgb(1.0, 0.9, 0.7),   // Warm white
        Color::srgb(1.0, 0.95, 0.8),  // Soft white
        Color::srgb(0.9, 0.85, 0.7),  // Warm yellow
        Color::srgb(0.7, 0.8, 1.0),   // Cool white (TV glow)
    ];

    let mut window_count = 0;

    for (_building, transform, mesh_handle) in building_query.iter() {
        // Get building dimensions from mesh AABB
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        let building_width = aabb.half_extents.x * 2.0;
        let building_height = aabb.half_extents.y * 2.0;
        let building_depth = aabb.half_extents.z * 2.0;

        // Skip very small buildings
        if building_height < config.floor_height * 1.5 {
            continue;
        }

        let num_floors = (building_height / config.floor_height).floor() as usize;
        let pos = transform.translation;

        // Calculate windows per side based on building width
        let windows_x = ((building_width / 3.0).floor() as usize).max(1).min(6);
        let windows_z = ((building_depth / 3.0).floor() as usize).max(1).min(6);

        // Spawn windows on all 4 sides
        for floor in 0..num_floors {
            let floor_y = pos.y - building_height / 2.0 + (floor as f32 + 0.5) * config.floor_height + 0.5;

            // Front and back faces (along X axis)
            for side in [-1.0, 1.0] {
                let face_z = pos.z + side * (building_depth / 2.0 + 0.05);

                for i in 0..windows_x {
                    let window_x = pos.x - building_width / 2.0
                        + (i as f32 + 0.5) * (building_width / windows_x as f32);

                    let occupied = rng.gen::<f32>() < config.occupancy_rate;
                    let color_idx = rng.gen_range(0..window_colors.len());
                    let intensity = 0.8 + rng.gen::<f32>() * 0.4;
                    let light_color = window_colors[color_idx].to_linear();

                    let material = materials.add(StandardMaterial {
                        base_color: Color::srgba(0.1, 0.1, 0.15, 0.8),
                        emissive: LinearRgba::BLACK, // Start dark, updated by system
                        ..default()
                    });

                    commands.spawn((
                        Mesh3d(window_mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_xyz(window_x, floor_y, face_z)
                            .with_rotation(Quat::from_rotation_y(if side > 0.0 { 0.0 } else { std::f32::consts::PI })),
                        WindowLight { occupied, intensity, color: light_color },
                    ));

                    window_count += 1;
                }
            }

            // Left and right faces (along Z axis)
            for side in [-1.0, 1.0] {
                let face_x = pos.x + side * (building_width / 2.0 + 0.05);

                for i in 0..windows_z {
                    let window_z = pos.z - building_depth / 2.0
                        + (i as f32 + 0.5) * (building_depth / windows_z as f32);

                    let occupied = rng.gen::<f32>() < config.occupancy_rate;
                    let color_idx = rng.gen_range(0..window_colors.len());
                    let intensity = 0.8 + rng.gen::<f32>() * 0.4;
                    let light_color = window_colors[color_idx].to_linear();

                    let material = materials.add(StandardMaterial {
                        base_color: Color::srgba(0.1, 0.1, 0.15, 0.8),
                        emissive: LinearRgba::BLACK, // Start dark, updated by system
                        ..default()
                    });

                    commands.spawn((
                        Mesh3d(window_mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_xyz(face_x, floor_y, window_z)
                            .with_rotation(Quat::from_rotation_y(
                                std::f32::consts::FRAC_PI_2 * if side > 0.0 { 1.0 } else { -1.0 }
                            )),
                        WindowLight { occupied, intensity, color: light_color },
                    ));

                    window_count += 1;
                }
            }
        }
    }

    info!("Spawned {} window lights", window_count);
}

fn update_window_brightness(
    tod: Res<TimeOfDay>,
    window_query: Query<(&WindowLight, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Calculate night factor (0 during day, 1 at night)
    let hour = tod.hour();
    let night_factor = if hour >= 6.0 && hour <= 7.0 {
        // Dawn - lights turning off
        1.0 - (hour - 6.0)
    } else if hour >= 18.0 && hour <= 19.0 {
        // Dusk - lights turning on
        hour - 18.0
    } else if hour > 7.0 && hour < 18.0 {
        // Day - lights off
        0.0
    } else {
        // Night - lights on
        1.0
    };

    // Update window materials
    for (window, material_handle) in window_query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if window.occupied && night_factor > 0.0 {
                // Use the stored window color
                material.emissive = window.color * window.intensity * night_factor * 3.0;
            } else {
                material.emissive = LinearRgba::BLACK;
            }
        }
    }
}

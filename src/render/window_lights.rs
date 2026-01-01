//! Window lights for buildings that illuminate at night.
//!
//! Now facade-aware: different window styles for Glass, Brick, Concrete, Metal, and Painted facades.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::FacadeStyle;
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

fn should_spawn_windows(spawned: Res<BuildingsSpawned>, window_query: Query<&WindowLight>) -> bool {
    spawned.0 && window_query.is_empty()
}

/// Window light component for nighttime illumination.
#[derive(Component)]
pub struct WindowLight {
    /// Whether this window is "occupied" (will light up at night)
    pub occupied: bool,
    /// Base emissive intensity when lit
    pub intensity: f32,
    /// Light color for this window
    pub color: LinearRgba,
}

/// Window frame component for traditional facades.
#[derive(Component)]
pub struct WindowFrame;

/// Configuration for window appearance based on facade style.
#[derive(Clone, Debug)]
pub struct FacadeWindowConfig {
    /// Window dimensions (width, height) in meters
    pub window_size: Vec2,
    /// Horizontal spacing between window centers
    pub horizontal_spacing: f32,
    /// Floor height / vertical spacing
    pub floor_height: f32,
    /// Frame visibility (0.0 = no frame, 1.0 = prominent)
    pub frame_visibility: f32,
    /// Frame width in meters
    pub frame_width: f32,
    /// Window occupancy rate for night lighting
    pub occupancy_rate: f32,
    /// Base window glass color
    pub glass_color: Color,
    /// Metallic property for glass
    pub metallic: f32,
    /// Roughness for glass surface
    pub roughness: f32,
    /// Frame color
    pub frame_color: Color,
    /// Night emissive intensity multiplier
    pub night_intensity: f32,
}

/// Get the window configuration for a facade style.
fn get_facade_config(facade: FacadeStyle) -> FacadeWindowConfig {
    match facade {
        FacadeStyle::Glass => FacadeWindowConfig {
            window_size: Vec2::new(2.4, 2.6),
            horizontal_spacing: 2.6,
            floor_height: 3.0,
            frame_visibility: 0.1,
            frame_width: 0.05,
            occupancy_rate: 0.75,
            glass_color: Color::srgba(0.3, 0.5, 0.7, 0.6),
            metallic: 0.8,
            roughness: 0.08,
            frame_color: Color::srgb(0.7, 0.7, 0.72),
            night_intensity: 4.0,
        },
        FacadeStyle::Brick => FacadeWindowConfig {
            window_size: Vec2::new(1.0, 1.4),
            horizontal_spacing: 2.5,
            floor_height: 3.0,
            frame_visibility: 1.0,
            frame_width: 0.1,
            occupancy_rate: 0.55,
            glass_color: Color::srgba(0.12, 0.12, 0.18, 0.75),
            metallic: 0.05,
            roughness: 0.3,
            frame_color: Color::srgb(0.95, 0.92, 0.85),
            night_intensity: 2.5,
        },
        FacadeStyle::Concrete => FacadeWindowConfig {
            window_size: Vec2::new(1.5, 1.8),
            horizontal_spacing: 2.2,
            floor_height: 3.5,
            frame_visibility: 0.3,
            frame_width: 0.06,
            occupancy_rate: 0.6,
            glass_color: Color::srgba(0.25, 0.28, 0.32, 0.65),
            metallic: 0.2,
            roughness: 0.15,
            frame_color: Color::srgb(0.4, 0.4, 0.42),
            night_intensity: 3.0,
        },
        FacadeStyle::Metal => FacadeWindowConfig {
            window_size: Vec2::new(2.0, 2.2),
            horizontal_spacing: 4.5,
            floor_height: 4.0,
            frame_visibility: 0.8,
            frame_width: 0.12,
            occupancy_rate: 0.3,
            glass_color: Color::srgba(0.18, 0.2, 0.22, 0.6),
            metallic: 0.4,
            roughness: 0.35,
            frame_color: Color::srgb(0.35, 0.35, 0.38),
            night_intensity: 2.0,
        },
        FacadeStyle::Painted => FacadeWindowConfig {
            window_size: Vec2::new(1.1, 1.5),
            horizontal_spacing: 2.3,
            floor_height: 3.0,
            frame_visibility: 1.0,
            frame_width: 0.1,
            occupancy_rate: 0.5,
            glass_color: Color::srgba(0.1, 0.1, 0.15, 0.75),
            metallic: 0.02,
            roughness: 0.25,
            frame_color: Color::srgb(0.98, 0.98, 0.95),
            night_intensity: 2.5,
        },
    }
}

#[derive(Resource)]
pub struct WindowLightConfig {
    pub seed: u64,
    pub enable_frames: bool,
}

impl Default for WindowLightConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            enable_frames: true,
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

    // Pre-create window meshes for each facade type
    let mut window_meshes: std::collections::HashMap<FacadeStyle, Handle<Mesh>> =
        std::collections::HashMap::new();
    let mut frame_meshes: std::collections::HashMap<FacadeStyle, Handle<Mesh>> =
        std::collections::HashMap::new();

    for facade in [
        FacadeStyle::Glass,
        FacadeStyle::Brick,
        FacadeStyle::Concrete,
        FacadeStyle::Metal,
        FacadeStyle::Painted,
    ] {
        let fc = get_facade_config(facade);
        window_meshes.insert(
            facade,
            meshes.add(Cuboid::new(fc.window_size.x, fc.window_size.y, 0.08)),
        );
        // Frame mesh is a border around the window
        if fc.frame_visibility >= 0.5 && config.enable_frames {
            // Create a simple frame as a slightly larger outline
            frame_meshes.insert(
                facade,
                meshes.add(Cuboid::new(
                    fc.window_size.x + fc.frame_width * 2.0,
                    fc.window_size.y + fc.frame_width * 2.0,
                    0.04,
                )),
            );
        }
    }

    // Night light colors for interior illumination
    let window_colors = [
        Color::srgb(1.0, 0.9, 0.7),  // Warm white
        Color::srgb(1.0, 0.95, 0.8), // Soft white
        Color::srgb(0.9, 0.85, 0.7), // Warm yellow
        Color::srgb(0.7, 0.8, 1.0),  // Cool white (TV glow)
    ];

    let mut window_count = 0;
    let mut frame_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
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

        let facade = building.facade_style;
        let fc = get_facade_config(facade);

        // Skip very small buildings
        if building_height < fc.floor_height * 1.5 {
            continue;
        }

        let num_floors = (building_height / fc.floor_height).floor() as usize;
        let pos = transform.translation;

        // Calculate windows per side based on building width and facade spacing
        let windows_x = ((building_width - 1.0) / fc.horizontal_spacing).floor() as usize;
        let windows_z = ((building_depth - 1.0) / fc.horizontal_spacing).floor() as usize;

        if windows_x == 0 && windows_z == 0 {
            continue;
        }

        let window_mesh = window_meshes.get(&facade).cloned().unwrap();
        let frame_mesh = frame_meshes.get(&facade).cloned();

        // Create glass material for this facade type
        let glass_material = materials.add(StandardMaterial {
            base_color: fc.glass_color,
            metallic: fc.metallic,
            perceptual_roughness: fc.roughness,
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::BLACK,
            ..default()
        });

        // Create frame material if needed
        let frame_material = if fc.frame_visibility >= 0.5 && config.enable_frames {
            Some(materials.add(StandardMaterial {
                base_color: fc.frame_color,
                perceptual_roughness: 0.7,
                metallic: if facade == FacadeStyle::Metal { 0.5 } else { 0.0 },
                ..default()
            }))
        } else {
            None
        };

        // Spawn windows on all 4 sides
        for floor in 0..num_floors {
            let floor_y =
                pos.y - building_height / 2.0 + (floor as f32 + 0.5) * fc.floor_height + 0.3;

            // Front and back faces (along X axis)
            for side in [-1.0_f32, 1.0] {
                let face_z = pos.z + side * (building_depth / 2.0 + 0.05);

                for i in 0..windows_x.max(1) {
                    let window_x = pos.x - building_width / 2.0
                        + fc.horizontal_spacing / 2.0
                        + i as f32 * fc.horizontal_spacing;

                    // Check if window is within building bounds
                    if (window_x - pos.x).abs() > building_width / 2.0 - fc.window_size.x / 2.0 {
                        continue;
                    }

                    let occupied = rng.gen::<f32>() < fc.occupancy_rate;
                    let color_idx = rng.gen_range(0..window_colors.len());
                    let intensity = fc.night_intensity * (0.8 + rng.gen::<f32>() * 0.4);
                    let light_color = window_colors[color_idx].to_linear();

                    let rotation =
                        Quat::from_rotation_y(if side > 0.0 { 0.0 } else { std::f32::consts::PI });

                    // Spawn frame first (behind window)
                    if let (Some(ref fm), Some(ref fmat)) = (&frame_mesh, &frame_material) {
                        commands.spawn((
                            Mesh3d(fm.clone()),
                            MeshMaterial3d(fmat.clone()),
                            Transform::from_xyz(window_x, floor_y, face_z - 0.02 * side)
                                .with_rotation(rotation),
                            WindowFrame,
                        ));
                        frame_count += 1;
                    }

                    // Spawn window
                    commands.spawn((
                        Mesh3d(window_mesh.clone()),
                        MeshMaterial3d(glass_material.clone()),
                        Transform::from_xyz(window_x, floor_y, face_z).with_rotation(rotation),
                        WindowLight {
                            occupied,
                            intensity,
                            color: light_color,
                        },
                    ));

                    window_count += 1;
                }
            }

            // Left and right faces (along Z axis)
            for side in [-1.0_f32, 1.0] {
                let face_x = pos.x + side * (building_width / 2.0 + 0.05);

                for i in 0..windows_z.max(1) {
                    let window_z = pos.z - building_depth / 2.0
                        + fc.horizontal_spacing / 2.0
                        + i as f32 * fc.horizontal_spacing;

                    // Check if window is within building bounds
                    if (window_z - pos.z).abs() > building_depth / 2.0 - fc.window_size.y / 2.0 {
                        continue;
                    }

                    let occupied = rng.gen::<f32>() < fc.occupancy_rate;
                    let color_idx = rng.gen_range(0..window_colors.len());
                    let intensity = fc.night_intensity * (0.8 + rng.gen::<f32>() * 0.4);
                    let light_color = window_colors[color_idx].to_linear();

                    let rotation = Quat::from_rotation_y(
                        std::f32::consts::FRAC_PI_2 * if side > 0.0 { 1.0 } else { -1.0 },
                    );

                    // Spawn frame first (behind window)
                    if let (Some(ref fm), Some(ref fmat)) = (&frame_mesh, &frame_material) {
                        commands.spawn((
                            Mesh3d(fm.clone()),
                            MeshMaterial3d(fmat.clone()),
                            Transform::from_xyz(face_x - 0.02 * side, floor_y, window_z)
                                .with_rotation(rotation),
                            WindowFrame,
                        ));
                        frame_count += 1;
                    }

                    // Spawn window
                    commands.spawn((
                        Mesh3d(window_mesh.clone()),
                        MeshMaterial3d(glass_material.clone()),
                        Transform::from_xyz(face_x, floor_y, window_z).with_rotation(rotation),
                        WindowLight {
                            occupied,
                            intensity,
                            color: light_color,
                        },
                    ));

                    window_count += 1;
                }
            }
        }
    }

    info!(
        "Spawned {} window lights, {} window frames",
        window_count, frame_count
    );
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
                material.emissive = window.color * window.intensity * night_factor;
            } else {
                material.emissive = LinearRgba::BLACK;
            }
        }
    }
}

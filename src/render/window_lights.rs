//! Window lights for buildings that illuminate at night.
//!
//! Uses shared StandardMaterial with emissive properties for GPU batching.
//! Windows glow at night based on time of day and occupancy.

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
            .init_resource::<WindowsSpawned>()
            .init_resource::<WindowMaterialPalette>()
            .add_systems(Startup, initialize_window_materials)
            .add_systems(Update, spawn_window_lights.run_if(should_spawn_windows))
            .add_systems(Update, update_window_emissive);
    }
}

/// Marker resource to prevent window system from running multiple times.
#[derive(Resource, Default)]
pub struct WindowsSpawned(pub bool);

fn should_spawn_windows(spawned: Res<BuildingsSpawned>, windows: Res<WindowsSpawned>) -> bool {
    spawned.0 && !windows.0
}

/// Component marking a window entity.
#[derive(Component)]
pub struct WindowPane {
    /// Whether this window is "occupied" (will light up at night)
    pub occupied: bool,
    /// Base emissive intensity when lit
    pub base_intensity: f32,
    /// Material index for updating emissive
    pub material_variant: usize,
}

/// Configuration for window appearance based on facade style.
#[derive(Clone, Debug)]
pub struct FacadeWindowConfig {
    pub window_size: Vec2,
    pub horizontal_spacing: f32,
    pub floor_height: f32,
    pub occupancy_rate: f32,
    pub night_intensity: f32,
}

fn get_facade_config(facade: FacadeStyle) -> FacadeWindowConfig {
    match facade {
        FacadeStyle::Glass => FacadeWindowConfig {
            window_size: Vec2::new(2.4, 2.6),
            horizontal_spacing: 2.6,
            floor_height: 3.0,
            occupancy_rate: 0.75,
            night_intensity: 4.0,
        },
        FacadeStyle::Brick => FacadeWindowConfig {
            window_size: Vec2::new(1.0, 1.4),
            horizontal_spacing: 2.5,
            floor_height: 3.0,
            occupancy_rate: 0.55,
            night_intensity: 2.5,
        },
        FacadeStyle::Concrete => FacadeWindowConfig {
            window_size: Vec2::new(1.5, 1.8),
            horizontal_spacing: 2.2,
            floor_height: 3.5,
            occupancy_rate: 0.6,
            night_intensity: 3.0,
        },
        FacadeStyle::Metal => FacadeWindowConfig {
            window_size: Vec2::new(2.0, 2.2),
            horizontal_spacing: 4.5,
            floor_height: 4.0,
            occupancy_rate: 0.3,
            night_intensity: 2.0,
        },
        FacadeStyle::Painted => FacadeWindowConfig {
            window_size: Vec2::new(1.1, 1.5),
            horizontal_spacing: 2.3,
            floor_height: 3.0,
            occupancy_rate: 0.5,
            night_intensity: 2.5,
        },
    }
}

#[derive(Resource)]
pub struct WindowLightConfig {
    pub seed: u64,
    pub max_windows: usize,
}

impl Default for WindowLightConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            max_windows: 50_000, // Reduced for performance with individual entities
        }
    }
}

/// Night light colors for interior illumination.
const WINDOW_COLORS: [Color; 4] = [
    Color::srgb(1.0, 0.9, 0.7),  // Warm white
    Color::srgb(1.0, 0.95, 0.8), // Soft white
    Color::srgb(0.9, 0.85, 0.7), // Warm yellow
    Color::srgb(0.7, 0.8, 1.0),  // Cool white (TV glow)
];

/// Shared window materials for GPU batching.
#[derive(Resource, Default)]
pub struct WindowMaterialPalette {
    /// Materials for each color variant (warm white, soft white, warm yellow, cool white)
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Shared window quad mesh
    pub quad_mesh: Handle<Mesh>,
}

fn initialize_window_materials(
    mut palette: ResMut<WindowMaterialPalette>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create shared quad mesh for all windows
    palette.quad_mesh = meshes.add(Cuboid::new(1.0, 1.0, 0.05));

    // Create materials for each color variant
    // Start with low emissive (daytime) - will be updated based on time of day
    for color in WINDOW_COLORS.iter() {
        let linear = color.to_linear();
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(linear.red * 0.3, linear.green * 0.3, linear.blue * 0.3, 0.85),
            emissive: LinearRgba::new(0.0, 0.0, 0.0, 1.0), // Starts dark
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        palette.materials.push(material);
    }

    info!("Window material palette initialized: {} variants", palette.materials.len());
}

fn spawn_window_lights(
    mut commands: Commands,
    config: Res<WindowLightConfig>,
    palette: Res<WindowMaterialPalette>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    meshes: Res<Assets<Mesh>>,
    mut spawned: ResMut<WindowsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning window lights...");

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut window_count = 0;
    let mut skipped_small = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        if window_count >= config.max_windows {
            break;
        }

        // Get building dimensions from mesh AABB
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        // Apply transform scale to get world-space dimensions
        let scale = transform.scale;
        let building_width = aabb.half_extents.x * 2.0 * scale.x;
        let building_height = aabb.half_extents.y * 2.0 * scale.y;
        let building_depth = aabb.half_extents.z * 2.0 * scale.z;

        let facade = building.facade_style;
        let fc = get_facade_config(facade);

        // Skip very small buildings
        if building_height < fc.floor_height * 1.5 {
            skipped_small += 1;
            continue;
        }

        let num_floors = (building_height / fc.floor_height).floor() as usize;
        let pos = transform.translation;

        // Calculate windows per side
        let windows_x = ((building_width - 1.0) / fc.horizontal_spacing).floor() as usize;
        let windows_z = ((building_depth - 1.0) / fc.horizontal_spacing).floor() as usize;

        if windows_x == 0 && windows_z == 0 {
            continue;
        }

        // Spawn windows on front and back faces (along X axis)
        for floor in 0..num_floors {
            let floor_y = pos.y - building_height / 2.0 + (floor as f32 + 0.5) * fc.floor_height + 0.3;

            for side in [-1.0_f32, 1.0] {
                let face_z = pos.z + side * (building_depth / 2.0 + 0.03);

                for i in 0..windows_x.max(1) {
                    if window_count >= config.max_windows {
                        break;
                    }

                    let window_x = pos.x - building_width / 2.0
                        + fc.horizontal_spacing / 2.0
                        + i as f32 * fc.horizontal_spacing;

                    if (window_x - pos.x).abs() > building_width / 2.0 - fc.window_size.x / 2.0 {
                        continue;
                    }

                    let occupied = rng.gen::<f32>() < fc.occupancy_rate;
                    let color_idx = rng.gen_range(0..WINDOW_COLORS.len());
                    let intensity = fc.night_intensity * (0.8 + rng.gen::<f32>() * 0.4);

                    // Use shared material for GPU batching
                    let material = palette.materials[color_idx].clone();

                    commands.spawn((
                        Mesh3d(palette.quad_mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_xyz(window_x, floor_y, face_z)
                            .with_scale(Vec3::new(fc.window_size.x, fc.window_size.y, 1.0))
                            .with_rotation(Quat::from_rotation_y(if side > 0.0 { 0.0 } else { std::f32::consts::PI })),
                        WindowPane {
                            occupied,
                            base_intensity: intensity,
                            material_variant: color_idx,
                        },
                    ));

                    window_count += 1;
                }
            }

            // Left and right faces (along Z axis)
            for side in [-1.0_f32, 1.0] {
                let face_x = pos.x + side * (building_width / 2.0 + 0.03);

                for i in 0..windows_z.max(1) {
                    if window_count >= config.max_windows {
                        break;
                    }

                    let window_z = pos.z - building_depth / 2.0
                        + fc.horizontal_spacing / 2.0
                        + i as f32 * fc.horizontal_spacing;

                    if (window_z - pos.z).abs() > building_depth / 2.0 - fc.window_size.y / 2.0 {
                        continue;
                    }

                    let occupied = rng.gen::<f32>() < fc.occupancy_rate;
                    let color_idx = rng.gen_range(0..WINDOW_COLORS.len());
                    let intensity = fc.night_intensity * (0.8 + rng.gen::<f32>() * 0.4);

                    let material = palette.materials[color_idx].clone();

                    commands.spawn((
                        Mesh3d(palette.quad_mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_xyz(face_x, floor_y, window_z)
                            .with_scale(Vec3::new(fc.window_size.x, fc.window_size.y, 1.0))
                            .with_rotation(Quat::from_rotation_y(
                                if side > 0.0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 }
                            )),
                        WindowPane {
                            occupied,
                            base_intensity: intensity,
                            material_variant: color_idx,
                        },
                    ));

                    window_count += 1;
                }
            }
        }
    }

    info!("Spawned {} window lights ({} buildings too small)", window_count, skipped_small);
}

/// Update window emissive based on time of day.
fn update_window_emissive(
    tod: Res<TimeOfDay>,
    palette: Res<WindowMaterialPalette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Only update when time changes significantly
    if !tod.is_changed() {
        return;
    }

    let hour = tod.hour();
    let night_factor = calculate_night_factor(hour);

    // Update all palette materials with current night factor
    for (i, handle) in palette.materials.iter().enumerate() {
        if let Some(material) = materials.get_mut(handle) {
            let base_color = WINDOW_COLORS[i].to_linear();

            // Bright emissive glow at night - visible against dark backdrop
            material.emissive = LinearRgba::new(
                base_color.red * night_factor * 8.0,
                base_color.green * night_factor * 8.0,
                base_color.blue * night_factor * 8.0,
                1.0,
            );
        }
    }
}

/// Calculate night factor (0.0 = day, 1.0 = night).
fn calculate_night_factor(hour: f32) -> f32 {
    if hour >= 6.0 && hour <= 8.0 {
        // Morning - windows turning off
        1.0 - (hour - 6.0) / 2.0
    } else if hour >= 17.0 && hour <= 19.0 {
        // Evening - windows turning on
        (hour - 17.0) / 2.0
    } else if hour > 8.0 && hour < 17.0 {
        // Day - minimal windows lit
        0.05
    } else {
        // Night - windows on
        0.7
    }
}

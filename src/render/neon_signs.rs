//! Neon signs for commercial buildings.
//!
//! Spawns glowing neon signs on commercial building facades that
//! illuminate at night. Uses emissive materials for GPU-efficient glow.

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::day_night::TimeOfDay;

pub struct NeonSignsPlugin;

impl Plugin for NeonSignsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NeonSignConfig>()
            .init_resource::<NeonSignsSpawned>()
            .init_resource::<NeonMaterialPalette>()
            .add_systems(Startup, initialize_neon_materials)
            .add_systems(Update, spawn_neon_signs.run_if(should_spawn_signs))
            .add_systems(Update, update_neon_glow);
    }
}

/// Marker resource to prevent neon signs from spawning multiple times.
#[derive(Resource, Default)]
pub struct NeonSignsSpawned(pub bool);

fn should_spawn_signs(spawned: Res<BuildingsSpawned>, neon_spawned: Res<NeonSignsSpawned>) -> bool {
    spawned.0 && !neon_spawned.0
}

/// Component marking a neon sign entity.
#[derive(Component)]
pub struct NeonSign {
    /// Color index into the palette
    pub color_index: usize,
    /// Base emissive intensity
    pub base_intensity: f32,
    /// Flicker phase offset (for subtle animation)
    pub flicker_phase: f32,
}

/// Component for the sign backing (dark background).
#[derive(Component)]
pub struct SignBacking;

/// Configuration for neon sign spawning.
#[derive(Resource)]
pub struct NeonSignConfig {
    pub seed: u64,
    /// Probability that a commercial building gets a neon sign
    pub sign_probability: f32,
    /// Probability of a second sign on the same building
    pub second_sign_probability: f32,
    /// Height above ground for signs
    pub sign_height: f32,
    /// Maximum sign width
    pub max_sign_width: f32,
    /// Sign thickness
    pub sign_depth: f32,
}

impl Default for NeonSignConfig {
    fn default() -> Self {
        Self {
            seed: 99999,
            sign_probability: 0.7,       // 70% of commercial buildings
            second_sign_probability: 0.3, // 30% chance of second sign
            sign_height: 4.0,            // 4m above ground
            max_sign_width: 6.0,         // Max 6m wide
            sign_depth: 0.15,            // 15cm thick
        }
    }
}

/// Neon sign colors - vibrant urban palette.
const NEON_COLORS: [(Color, &str); 8] = [
    (Color::srgb(1.0, 0.2, 0.3), "red"),        // Hot red
    (Color::srgb(0.2, 0.6, 1.0), "blue"),       // Electric blue
    (Color::srgb(0.2, 1.0, 0.4), "green"),      // Neon green
    (Color::srgb(1.0, 0.4, 0.8), "pink"),       // Hot pink
    (Color::srgb(1.0, 0.9, 0.2), "yellow"),     // Yellow
    (Color::srgb(1.0, 0.5, 0.1), "orange"),     // Orange
    (Color::srgb(0.7, 0.3, 1.0), "purple"),     // Purple
    (Color::srgb(0.2, 1.0, 1.0), "cyan"),       // Cyan
];

/// Sign shape variations.
#[derive(Clone, Copy)]
enum SignShape {
    /// Wide horizontal rectangle
    Horizontal,
    /// Tall vertical rectangle
    Vertical,
    /// Square
    Square,
}

/// Shared neon materials for GPU batching.
#[derive(Resource, Default)]
pub struct NeonMaterialPalette {
    /// Materials for each neon color
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Dark backing material
    pub backing_material: Handle<StandardMaterial>,
    /// Shared quad mesh
    pub quad_mesh: Handle<Mesh>,
}

fn initialize_neon_materials(
    mut palette: ResMut<NeonMaterialPalette>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create shared quad mesh for all signs
    palette.quad_mesh = meshes.add(Cuboid::new(1.0, 1.0, 0.1));

    // Dark backing material
    palette.backing_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.12),
        perceptual_roughness: 0.8,
        metallic: 0.2,
        ..default()
    });

    // Create emissive materials for each neon color
    // Start with low emissive (daytime) - will be updated based on time of day
    for (color, _name) in NEON_COLORS.iter() {
        let linear = color.to_linear();
        let material = materials.add(StandardMaterial {
            base_color: *color,
            emissive: LinearRgba::new(linear.red * 0.1, linear.green * 0.1, linear.blue * 0.1, 1.0),
            ..default()
        });
        palette.materials.push(material);
    }

    info!("Neon sign palette initialized: {} colors", palette.materials.len());
}

fn spawn_neon_signs(
    mut commands: Commands,
    config: Res<NeonSignConfig>,
    palette: Res<NeonMaterialPalette>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    meshes: Res<Assets<Mesh>>,
    mut spawned: ResMut<NeonSignsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning neon signs on commercial buildings...");

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut sign_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Only commercial buildings get neon signs
        if building.building_type != BuildingArchetype::Commercial {
            continue;
        }

        // Random chance for this building to have a sign
        if rng.gen::<f32>() > config.sign_probability {
            continue;
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

        // Skip very small buildings
        if building_width < 4.0 || building_height < 6.0 {
            continue;
        }

        let pos = transform.translation;

        // Determine number of signs (1-2)
        let num_signs = if rng.gen::<f32>() < config.second_sign_probability { 2 } else { 1 };

        // Track which faces we've used
        let mut used_faces: Vec<usize> = Vec::new();

        for _ in 0..num_signs {
            // Pick a face (0=front +Z, 1=back -Z, 2=left -X, 3=right +X)
            let available_faces: Vec<usize> = (0..4).filter(|f| !used_faces.contains(f)).collect();
            if available_faces.is_empty() {
                break;
            }
            let face_idx = available_faces[rng.gen_range(0..available_faces.len())];
            used_faces.push(face_idx);

            // Determine face dimensions and position
            let (face_width, face_pos, face_rotation) = match face_idx {
                0 => (
                    building_width,
                    Vec3::new(pos.x, pos.y, pos.z + building_depth / 2.0 + 0.05),
                    Quat::IDENTITY,
                ),
                1 => (
                    building_width,
                    Vec3::new(pos.x, pos.y, pos.z - building_depth / 2.0 - 0.05),
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
                2 => (
                    building_depth,
                    Vec3::new(pos.x - building_width / 2.0 - 0.05, pos.y, pos.z),
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ),
                _ => (
                    building_depth,
                    Vec3::new(pos.x + building_width / 2.0 + 0.05, pos.y, pos.z),
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ),
            };

            // Calculate sign height (relative to building base, not center)
            let building_base = pos.y - building_height / 2.0;
            let sign_y = building_base + config.sign_height;

            // Skip if sign would be above building
            if sign_y > pos.y + building_height / 2.0 - 1.0 {
                continue;
            }

            // Pick sign shape and size
            let shape = match rng.gen_range(0..3) {
                0 => SignShape::Horizontal,
                1 => SignShape::Vertical,
                _ => SignShape::Square,
            };

            let max_width = (face_width * 0.6).min(config.max_sign_width);
            let (sign_width, sign_height) = match shape {
                SignShape::Horizontal => (
                    max_width * (0.6 + rng.gen::<f32>() * 0.4),
                    1.0 + rng.gen::<f32>() * 0.5,
                ),
                SignShape::Vertical => (
                    1.2 + rng.gen::<f32>() * 0.8,
                    2.0 + rng.gen::<f32>() * 1.5,
                ),
                SignShape::Square => {
                    let size = 1.5 + rng.gen::<f32>() * 1.0;
                    (size, size)
                }
            };

            // Pick a neon color
            let color_index = rng.gen_range(0..NEON_COLORS.len());
            let neon_material = palette.materials[color_index].clone();

            // Random offset from center of face
            let offset_range = (face_width / 2.0 - sign_width / 2.0 - 0.5).max(0.0);
            let horizontal_offset = if offset_range > 0.01 {
                rng.gen_range(-offset_range..offset_range)
            } else {
                0.0
            };

            // Calculate final sign position
            let local_offset = face_rotation * Vec3::new(horizontal_offset, 0.0, 0.0);
            let sign_pos = Vec3::new(
                face_pos.x + local_offset.x,
                sign_y,
                face_pos.z + local_offset.z,
            );

            // Spawn sign backing (slightly larger, behind the neon)
            commands.spawn((
                Mesh3d(palette.quad_mesh.clone()),
                MeshMaterial3d(palette.backing_material.clone()),
                Transform::from_translation(sign_pos - face_rotation * Vec3::Z * 0.03)
                    .with_rotation(face_rotation)
                    .with_scale(Vec3::new(sign_width + 0.3, sign_height + 0.2, config.sign_depth)),
                SignBacking,
            ));

            // Spawn the neon sign
            let flicker_phase = rng.gen::<f32>() * std::f32::consts::TAU;
            let base_intensity = 4.0 + rng.gen::<f32>() * 3.0;

            commands.spawn((
                Mesh3d(palette.quad_mesh.clone()),
                MeshMaterial3d(neon_material),
                Transform::from_translation(sign_pos)
                    .with_rotation(face_rotation)
                    .with_scale(Vec3::new(sign_width, sign_height, config.sign_depth * 0.5)),
                NeonSign {
                    color_index,
                    base_intensity,
                    flicker_phase,
                },
            ));

            sign_count += 1;
        }
    }

    info!("Spawned {} neon signs on commercial buildings", sign_count);
}

/// Update neon sign glow based on time of day.
fn update_neon_glow(
    tod: Res<TimeOfDay>,
    palette: Res<NeonMaterialPalette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    // Only update when time changes significantly
    if !tod.is_changed() {
        return;
    }

    let hour = tod.hour();
    let night_factor = calculate_night_factor(hour);

    // Add subtle flicker effect
    let elapsed = time.elapsed_secs();
    let flicker = 0.95 + 0.05 * (elapsed * 3.0).sin();

    // Update all palette materials with current night factor
    for (i, handle) in palette.materials.iter().enumerate() {
        if let Some(material) = materials.get_mut(handle) {
            let (base_color, _) = NEON_COLORS[i];
            let linear = base_color.to_linear();

            // Bright emissive glow at night - visible against dark backdrop
            let intensity = night_factor * 15.0 * flicker;
            material.emissive = LinearRgba::new(
                linear.red * intensity,
                linear.green * intensity,
                linear.blue * intensity,
                1.0,
            );
        }
    }
}

/// Calculate night factor (0.0 = day, 1.0 = night).
fn calculate_night_factor(hour: f32) -> f32 {
    if hour >= 5.0 && hour <= 7.0 {
        // Morning - signs turning off
        1.0 - (hour - 5.0) / 2.0
    } else if hour >= 17.0 && hour <= 19.0 {
        // Evening - signs turning on
        (hour - 17.0) / 2.0
    } else if hour > 7.0 && hour < 17.0 {
        // Day - signs mostly off (very dim)
        0.05
    } else {
        // Night - signs fully on
        1.0
    }
}

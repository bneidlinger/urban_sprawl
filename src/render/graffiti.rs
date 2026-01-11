//! Graffiti tags on industrial buildings.
//!
//! Spawns colorful graffiti tags on the walls of industrial buildings
//! at ground level, adding urban character to the city.

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct GraffitiPlugin;

impl Plugin for GraffitiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraffitiConfig>()
            .init_resource::<GraffitiSpawned>()
            .add_systems(Update, spawn_graffiti.run_if(should_spawn_graffiti));
    }
}

/// Marker resource to prevent graffiti system from running multiple times.
#[derive(Resource, Default)]
pub struct GraffitiSpawned(pub bool);

fn should_spawn_graffiti(
    buildings_spawned: Res<BuildingsSpawned>,
    graffiti_spawned: Res<GraffitiSpawned>,
) -> bool {
    buildings_spawned.0 && !graffiti_spawned.0
}

/// Graffiti tag marker component.
#[derive(Component)]
pub struct GraffitiTag {
    /// Color index into the palette.
    pub color_index: usize,
}

/// Configuration for graffiti spawning.
#[derive(Resource)]
pub struct GraffitiConfig {
    pub seed: u64,
    /// Probability of an industrial building getting graffiti.
    pub building_probability: f32,
    /// Minimum building height for graffiti placement.
    pub min_building_height: f32,
    /// Maximum height for graffiti placement (from ground).
    pub max_graffiti_height: f32,
}

impl Default for GraffitiConfig {
    fn default() -> Self {
        Self {
            seed: 66666,
            building_probability: 0.5,
            min_building_height: 4.0,
            max_graffiti_height: 4.0,
        }
    }
}

/// Neon spray paint colors for graffiti.
const GRAFFITI_COLORS: &[(f32, f32, f32)] = &[
    (1.0, 0.2, 0.6),   // Hot pink
    (0.4, 1.0, 0.3),   // Lime green
    (0.2, 0.6, 1.0),   // Electric blue
    (1.0, 0.5, 0.1),   // Orange
    (0.7, 0.2, 1.0),   // Purple
    (1.0, 1.0, 0.2),   // Yellow
    (0.2, 1.0, 0.9),   // Cyan
    (1.0, 0.3, 0.3),   // Coral red
];

fn spawn_graffiti(
    mut commands: Commands,
    config: Res<GraffitiConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graffiti_spawned: ResMut<GraffitiSpawned>,
) {
    graffiti_spawned.0 = true;

    info!("Spawning graffiti...");
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Pre-create graffiti materials with slight emissive for spray paint sheen
    let mut graffiti_materials: Vec<Handle<StandardMaterial>> = Vec::new();
    for &(r, g, b) in GRAFFITI_COLORS {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::new(r * 0.15, g * 0.15, b * 0.15, 1.0),
            perceptual_roughness: 0.6,
            metallic: 0.0,
            ..default()
        });
        graffiti_materials.push(material);
    }

    let mut graffiti_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Only industrial buildings get graffiti
        if building.building_type != BuildingArchetype::Industrial {
            continue;
        }

        // Random chance per building
        if rng.gen::<f32>() > config.building_probability {
            continue;
        }

        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        let scale = transform.scale;
        let building_height = aabb.half_extents.y * 2.0 * scale.y;
        let building_width = aabb.half_extents.x * 2.0 * scale.x;
        let building_depth = aabb.half_extents.z * 2.0 * scale.z;
        let pos = transform.translation;
        let base_y = pos.y - building_height / 2.0;

        if building_height < config.min_building_height {
            continue;
        }

        // Spawn 1-3 graffiti tags per building
        let num_tags = rng.gen_range(1..=3);

        for _ in 0..num_tags {
            let color_idx = rng.gen_range(0..GRAFFITI_COLORS.len());

            // Random size for variety
            let width = rng.gen_range(1.5..3.5);
            let height = rng.gen_range(1.0..2.5);

            // Create mesh for this tag
            let tag_mesh = meshes.add(Cuboid::new(width, height, 0.05));

            // Pick a random wall face
            let face = rng.gen_range(0..4);
            let (offset_x, offset_z, rotation) = match face {
                0 => (
                    building_width / 2.0 + 0.03,
                    rng.gen_range(-building_depth / 3.0..building_depth / 3.0),
                    Quat::from_rotation_y(PI / 2.0),
                ),
                1 => (
                    -building_width / 2.0 - 0.03,
                    rng.gen_range(-building_depth / 3.0..building_depth / 3.0),
                    Quat::from_rotation_y(-PI / 2.0),
                ),
                2 => (
                    rng.gen_range(-building_width / 3.0..building_width / 3.0),
                    building_depth / 2.0 + 0.03,
                    Quat::IDENTITY,
                ),
                _ => (
                    rng.gen_range(-building_width / 3.0..building_width / 3.0),
                    -building_depth / 2.0 - 0.03,
                    Quat::from_rotation_y(PI),
                ),
            };

            // Place at ground level (1-4m height)
            let graffiti_y = base_y + rng.gen_range(1.5..config.max_graffiti_height);

            commands.spawn((
                Mesh3d(tag_mesh),
                MeshMaterial3d(graffiti_materials[color_idx].clone()),
                Transform::from_xyz(pos.x + offset_x, graffiti_y, pos.z + offset_z)
                    .with_rotation(rotation),
                GraffitiTag {
                    color_index: color_idx,
                },
            ));

            graffiti_count += 1;
        }
    }

    info!("Spawned {} graffiti tags on industrial buildings", graffiti_count);
}

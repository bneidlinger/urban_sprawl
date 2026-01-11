//! Billboards and advertisements on commercial buildings.
//!
//! Spawns rooftop billboards and wall-mounted ad panels on commercial
//! buildings with night-time illumination.

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::clustered_shading::{DynamicCityLight, LightType};

pub struct BillboardsPlugin;

impl Plugin for BillboardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BillboardConfig>()
            .init_resource::<BillboardsSpawned>()
            .add_systems(Update, spawn_billboards.run_if(should_spawn_billboards));
    }
}

/// Marker resource to prevent billboard system from running multiple times.
#[derive(Resource, Default)]
pub struct BillboardsSpawned(pub bool);

fn should_spawn_billboards(
    buildings_spawned: Res<BuildingsSpawned>,
    billboards_spawned: Res<BillboardsSpawned>,
) -> bool {
    buildings_spawned.0 && !billboards_spawned.0
}

/// Billboard marker component.
#[derive(Component)]
pub struct Billboard {
    /// Whether this billboard is illuminated at night.
    pub lit: bool,
    /// Color index for identification.
    pub color_index: usize,
}

/// Configuration for billboard spawning.
#[derive(Resource)]
pub struct BillboardConfig {
    pub seed: u64,
    /// Probability of a commercial building getting a rooftop billboard.
    pub rooftop_probability: f32,
    /// Probability of a commercial building getting a wall billboard.
    pub wall_probability: f32,
    /// Minimum building height for rooftop billboards.
    pub min_height_rooftop: f32,
    /// Minimum building height for wall billboards.
    pub min_height_wall: f32,
}

impl Default for BillboardConfig {
    fn default() -> Self {
        Self {
            seed: 55555,
            rooftop_probability: 0.25,
            wall_probability: 0.35,
            min_height_rooftop: 12.0,
            min_height_wall: 8.0,
        }
    }
}

/// Billboard advertisement colors - bright, attention-grabbing.
const BILLBOARD_COLORS: &[(f32, f32, f32)] = &[
    (0.9, 0.2, 0.2),   // Red
    (0.2, 0.4, 0.9),   // Blue
    (0.95, 0.85, 0.2), // Yellow
    (0.2, 0.8, 0.3),   // Green
    (0.95, 0.5, 0.1),  // Orange
    (0.95, 0.95, 0.95),// White
    (0.2, 0.85, 0.9),  // Cyan
    (0.9, 0.3, 0.7),   // Magenta/Pink
];

fn spawn_billboards(
    mut commands: Commands,
    config: Res<BillboardConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut billboards_spawned: ResMut<BillboardsSpawned>,
) {
    billboards_spawned.0 = true;

    info!("Spawning billboards...");
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Pre-create billboard materials (one per color, lit and unlit variants)
    let mut billboard_materials: Vec<Handle<StandardMaterial>> = Vec::new();
    for &(r, g, b) in BILLBOARD_COLORS {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::new(r * 0.3, g * 0.3, b * 0.3, 1.0),
            perceptual_roughness: 0.3,
            ..default()
        });
        billboard_materials.push(material);
    }

    // Billboard meshes
    let rooftop_billboard_mesh = meshes.add(Cuboid::new(8.0, 4.0, 0.3));
    let wall_billboard_mesh = meshes.add(Cuboid::new(4.0, 6.0, 0.2));

    // Support pole for rooftop billboards
    let pole_mesh = meshes.add(Cylinder::new(0.15, 3.0));
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.32),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });

    let mut rooftop_count = 0;
    let mut wall_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Only commercial buildings get billboards
        if building.building_type != BuildingArchetype::Commercial {
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

        // Rooftop billboards on taller buildings
        if building_height >= config.min_height_rooftop
            && rng.gen::<f32>() < config.rooftop_probability
        {
            let color_idx = rng.gen_range(0..BILLBOARD_COLORS.len());
            let rooftop_y = pos.y + building_height / 2.0;

            // Pole
            commands.spawn((
                Mesh3d(pole_mesh.clone()),
                MeshMaterial3d(pole_material.clone()),
                Transform::from_xyz(pos.x, rooftop_y + 1.5, pos.z),
            ));

            // Billboard panel
            let billboard_y = rooftop_y + 3.0 + 2.0; // pole height + half billboard height
            let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);

            commands.spawn((
                Mesh3d(rooftop_billboard_mesh.clone()),
                MeshMaterial3d(billboard_materials[color_idx].clone()),
                Transform::from_xyz(pos.x, billboard_y, pos.z)
                    .with_rotation(rotation),
                Billboard {
                    lit: true,
                    color_index: color_idx,
                },
                DynamicCityLight {
                    light_type: LightType::Window,
                    base_intensity: 3.0,
                    current_factor: 0.0,
                },
            ));

            rooftop_count += 1;
        }

        // Wall billboards on medium+ buildings
        if building_height >= config.min_height_wall
            && rng.gen::<f32>() < config.wall_probability
        {
            let color_idx = rng.gen_range(0..BILLBOARD_COLORS.len());

            // Pick a random wall face
            let face = rng.gen_range(0..4);
            let (offset_x, offset_z, rotation) = match face {
                0 => (building_width / 2.0 + 0.15, 0.0, Quat::from_rotation_y(PI / 2.0)),
                1 => (-building_width / 2.0 - 0.15, 0.0, Quat::from_rotation_y(-PI / 2.0)),
                2 => (0.0, building_depth / 2.0 + 0.15, Quat::IDENTITY),
                _ => (0.0, -building_depth / 2.0 - 0.15, Quat::from_rotation_y(PI)),
            };

            // Place in upper half of building
            let billboard_y = base_y + building_height * 0.65;

            let is_lit = rng.gen::<f32>() < 0.7; // 70% are lit at night
            let mut entity_commands = commands.spawn((
                Mesh3d(wall_billboard_mesh.clone()),
                MeshMaterial3d(billboard_materials[color_idx].clone()),
                Transform::from_xyz(pos.x + offset_x, billboard_y, pos.z + offset_z)
                    .with_rotation(rotation),
                Billboard {
                    lit: is_lit,
                    color_index: color_idx,
                },
            ));

            // Add night illumination for lit wall billboards
            if is_lit {
                entity_commands.insert(DynamicCityLight {
                    light_type: LightType::Window,
                    base_intensity: 2.0,
                    current_factor: 0.0,
                });
            }

            wall_count += 1;
        }
    }

    info!("Spawned {} rooftop billboards and {} wall billboards", rooftop_count, wall_count);
}

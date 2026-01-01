//! Rooftop details: AC units, water towers, antennas, and helipads.
//!
//! Spawns procedural details on building rooftops based on building type,
//! facade style, and height. Uses the same AABB-based approach as window_lights.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::{BuildingArchetype, FacadeStyle};
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct RooftopDetailsPlugin;

impl Plugin for RooftopDetailsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RooftopDetailConfig>()
            .add_systems(Update, spawn_rooftop_details.run_if(should_spawn_details));
    }
}

fn should_spawn_details(spawned: Res<BuildingsSpawned>, detail_query: Query<&RooftopDetail>) -> bool {
    spawned.0 && detail_query.is_empty()
}

/// Marker for all rooftop details.
#[derive(Component)]
pub struct RooftopDetail;

/// HVAC / AC unit on rooftop.
#[derive(Component)]
pub struct ACUnit;

/// Water tower (traditional residential).
#[derive(Component)]
pub struct WaterTower;

/// Antenna / communication tower.
#[derive(Component)]
pub struct Antenna;

/// Helipad on tall buildings.
#[derive(Component)]
pub struct Helipad;

/// Configuration for rooftop detail spawning.
#[derive(Resource)]
pub struct RooftopDetailConfig {
    pub seed: u64,
    /// Probability of AC units on commercial/industrial buildings.
    pub ac_unit_probability: f32,
    /// Probability of water towers on traditional residential.
    pub water_tower_probability: f32,
    /// Probability of antennas on any building.
    pub antenna_probability: f32,
    /// Minimum building height for helipads (meters).
    pub helipad_min_height: f32,
    /// Probability of helipad on tall commercial buildings.
    pub helipad_probability: f32,
}

impl Default for RooftopDetailConfig {
    fn default() -> Self {
        Self {
            seed: 88888,
            ac_unit_probability: 0.6,
            water_tower_probability: 0.35,
            antenna_probability: 0.2,
            helipad_min_height: 20.0, // ~6+ floors
            helipad_probability: 0.25,
        }
    }
}

fn spawn_rooftop_details(
    mut commands: Commands,
    config: Res<RooftopDetailConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning rooftop details...");

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Pre-create meshes
    let ac_unit_mesh = meshes.add(Cuboid::new(1.0, 0.6, 0.8));
    let ac_unit_top_mesh = meshes.add(Cuboid::new(0.9, 0.05, 0.7)); // Grille on top
    let water_tower_tank_mesh = meshes.add(Cylinder::new(1.0, 2.5));
    let water_tower_leg_mesh = meshes.add(Cylinder::new(0.08, 3.0));
    let antenna_mesh = meshes.add(Cylinder::new(0.04, 1.0)); // Will be scaled per-building
    let helipad_mesh = meshes.add(Cylinder::new(3.0, 0.1));

    // Materials
    let ac_unit_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.72),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });

    let ac_unit_grille_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.32),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let water_tower_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.32, 0.25), // Rusty brown
        perceptual_roughness: 0.8,
        ..default()
    });

    let water_tower_leg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let antenna_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.42),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    let helipad_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.6,
        ..default()
    });

    let mut ac_count = 0;
    let mut water_tower_count = 0;
    let mut antenna_count = 0;
    let mut helipad_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        let building_height = aabb.half_extents.y * 2.0;
        let building_width = aabb.half_extents.x * 2.0;
        let building_depth = aabb.half_extents.z * 2.0;

        // Skip very small buildings
        if building_height < 6.0 || building_width < 4.0 || building_depth < 4.0 {
            continue;
        }

        let pos = transform.translation;
        let rooftop_y = pos.y + building_height / 2.0;

        // Rooftop usable area (inset from edges)
        let inset = 1.5;
        let usable_width = (building_width - inset * 2.0).max(1.0);
        let usable_depth = (building_depth - inset * 2.0).max(1.0);

        // Track what's been placed to avoid overlaps
        let mut has_helipad = false;
        let mut _has_water_tower = false;

        // Helipads - only on tall commercial buildings
        if building.building_type == BuildingArchetype::Commercial
            && building_height >= config.helipad_min_height
            && rng.gen::<f32>() < config.helipad_probability
        {
            // Only add helipad if roof is large enough
            if usable_width >= 4.0 && usable_depth >= 4.0 {
                commands.spawn((
                    Mesh3d(helipad_mesh.clone()),
                    MeshMaterial3d(helipad_material.clone()),
                    Transform::from_xyz(pos.x, rooftop_y + 0.05, pos.z),
                    RooftopDetail,
                    Helipad,
                ));
                helipad_count += 1;
                has_helipad = true;
            }
        }

        // Water towers - traditional residential buildings (Brick or Painted)
        if !has_helipad
            && building.building_type == BuildingArchetype::Residential
            && (building.facade_style == FacadeStyle::Brick
                || building.facade_style == FacadeStyle::Painted)
            && building_height >= 6.0 // At least 2 floors
            && rng.gen::<f32>() < config.water_tower_probability
        {
            // Place in a corner
            let corner_x = if rng.gen::<bool>() {
                pos.x + usable_width / 2.0 - 1.5
            } else {
                pos.x - usable_width / 2.0 + 1.5
            };
            let corner_z = if rng.gen::<bool>() {
                pos.z + usable_depth / 2.0 - 1.5
            } else {
                pos.z - usable_depth / 2.0 + 1.5
            };

            let leg_height = 3.0;
            let tank_bottom = rooftop_y + leg_height;

            // Tank
            commands.spawn((
                Mesh3d(water_tower_tank_mesh.clone()),
                MeshMaterial3d(water_tower_material.clone()),
                Transform::from_xyz(corner_x, tank_bottom + 1.25, corner_z),
                RooftopDetail,
                WaterTower,
            ));

            // 4 legs
            let leg_offsets = [
                (0.6, 0.6),
                (0.6, -0.6),
                (-0.6, 0.6),
                (-0.6, -0.6),
            ];
            for (dx, dz) in leg_offsets {
                commands.spawn((
                    Mesh3d(water_tower_leg_mesh.clone()),
                    MeshMaterial3d(water_tower_leg_material.clone()),
                    Transform::from_xyz(corner_x + dx, rooftop_y + leg_height / 2.0, corner_z + dz),
                    RooftopDetail,
                ));
            }

            water_tower_count += 1;
            _has_water_tower = true;
        }

        // AC Units - commercial and industrial buildings
        if !has_helipad
            && (building.building_type == BuildingArchetype::Commercial
                || building.building_type == BuildingArchetype::Industrial)
            && rng.gen::<f32>() < config.ac_unit_probability
        {
            // Number of units based on building size
            let max_units = ((usable_width * usable_depth) / 12.0).floor() as usize;
            let num_units = rng.gen_range(2..=max_units.clamp(2, 6));

            // Grid-based placement with some randomness
            let mut placed_positions: Vec<Vec2> = Vec::new();

            for _ in 0..num_units {
                // Try to find a valid position
                for _ in 0..10 {
                    let offset_x = rng.gen_range(-usable_width / 2.0..usable_width / 2.0);
                    let offset_z = rng.gen_range(-usable_depth / 2.0..usable_depth / 2.0);
                    let test_pos = Vec2::new(offset_x, offset_z);

                    // Check spacing from other AC units
                    let too_close = placed_positions
                        .iter()
                        .any(|p| p.distance(test_pos) < 2.0);

                    if !too_close {
                        let ac_x = pos.x + offset_x;
                        let ac_z = pos.z + offset_z;

                        // AC unit body
                        commands.spawn((
                            Mesh3d(ac_unit_mesh.clone()),
                            MeshMaterial3d(ac_unit_material.clone()),
                            Transform::from_xyz(ac_x, rooftop_y + 0.3, ac_z),
                            RooftopDetail,
                            ACUnit,
                        ));

                        // Grille on top
                        commands.spawn((
                            Mesh3d(ac_unit_top_mesh.clone()),
                            MeshMaterial3d(ac_unit_grille_material.clone()),
                            Transform::from_xyz(ac_x, rooftop_y + 0.62, ac_z),
                            RooftopDetail,
                        ));

                        placed_positions.push(test_pos);
                        ac_count += 1;
                        break;
                    }
                }
            }
        }

        // Antennas - any building type
        if !has_helipad && rng.gen::<f32>() < config.antenna_probability {
            // Height proportional to building height
            let antenna_height = (building_height * 0.15).clamp(2.0, 8.0);

            // Place near center or corner
            let (antenna_x, antenna_z) = if rng.gen::<f32>() < 0.5 {
                // Center
                (pos.x, pos.z)
            } else {
                // Corner
                let cx = if rng.gen::<bool>() { 1.0 } else { -1.0 };
                let cz = if rng.gen::<bool>() { 1.0 } else { -1.0 };
                (
                    pos.x + cx * (usable_width / 2.0 - 0.5),
                    pos.z + cz * (usable_depth / 2.0 - 0.5),
                )
            };

            commands.spawn((
                Mesh3d(antenna_mesh.clone()),
                MeshMaterial3d(antenna_material.clone()),
                Transform::from_xyz(antenna_x, rooftop_y + antenna_height / 2.0, antenna_z)
                    .with_scale(Vec3::new(1.0, antenna_height, 1.0)),
                RooftopDetail,
                Antenna,
            ));

            antenna_count += 1;
        }
    }

    info!(
        "Spawned rooftop details: {} AC units, {} water towers, {} antennas, {} helipads",
        ac_count, water_tower_count, antenna_count, helipad_count
    );
}

//! Multi-story parking garage structures near commercial areas.
//!
//! Spawns parking garages with multiple floors, ramps, and vehicle openings.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct ParkingGaragesPlugin;

impl Plugin for ParkingGaragesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParkingGarageConfig>()
            .init_resource::<ParkingGaragesSpawned>()
            .add_systems(Update, spawn_parking_garages.run_if(should_spawn_garages));
    }
}

#[derive(Resource, Default)]
pub struct ParkingGaragesSpawned(pub bool);

fn should_spawn_garages(
    buildings_spawned: Res<BuildingsSpawned>,
    garages_spawned: Res<ParkingGaragesSpawned>,
) -> bool {
    buildings_spawned.0 && !garages_spawned.0
}

/// Parking garage marker component.
#[derive(Component)]
pub struct ParkingGarage {
    pub floors: u32,
    pub capacity: u32,
}

#[derive(Resource)]
pub struct ParkingGarageConfig {
    pub seed: u64,
    pub max_garages: usize,
    pub min_spacing: f32,
    pub min_floors: u32,
    pub max_floors: u32,
    pub floor_height: f32,
    pub width: f32,
    pub depth: f32,
    pub ramp_width: f32,
}

impl Default for ParkingGarageConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            max_garages: 6,
            min_spacing: 80.0,
            min_floors: 3,
            max_floors: 6,
            floor_height: 3.0,
            width: 25.0,
            depth: 18.0,
            ramp_width: 5.0,
        }
    }
}

fn spawn_parking_garages(
    mut commands: Commands,
    config: Res<ParkingGarageConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<ParkingGaragesSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut garage_count = 0;
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Collect commercial building positions (garages near commercial areas)
    let mut eligible_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| b.building_type == BuildingArchetype::Commercial)
        .map(|(_, t)| t.translation)
        .collect();

    // Shuffle
    for i in (1..eligible_positions.len()).rev() {
        let j = rng.gen_range(0..=i);
        eligible_positions.swap(i, j);
    }

    // Materials
    let concrete_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.53),
        perceptual_roughness: 0.9,
        ..default()
    });

    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.95,
        ..default()
    });

    let barrier_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.85),
        perceptual_roughness: 0.7,
        ..default()
    });

    let ramp_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.42),
        perceptual_roughness: 0.85,
        ..default()
    });

    let stripe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.8, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });

    for pos in eligible_positions {
        if garage_count >= config.max_garages {
            break;
        }

        // Offset from building
        let offset = Vec3::new(
            rng.gen_range(-20.0..20.0),
            0.0,
            rng.gen_range(-20.0..20.0),
        );
        let garage_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        // Check spacing
        let too_close = placed_positions
            .iter()
            .any(|p| p.distance(garage_pos) < config.min_spacing);
        if too_close {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen_range(0..4) as f32 * PI / 2.0);
        let floors = rng.gen_range(config.min_floors..=config.max_floors);
        let total_height = floors as f32 * config.floor_height;
        let capacity = floors * 40; // ~40 spaces per floor

        commands
            .spawn((
                Transform::from_translation(garage_pos).with_rotation(rotation),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                ParkingGarage { floors, capacity },
            ))
            .with_children(|parent| {
                // Create each floor
                for floor in 0..floors {
                    let floor_y = floor as f32 * config.floor_height;

                    // Floor slab
                    let slab_mesh = meshes.add(Cuboid::new(config.width, 0.2, config.depth));
                    parent.spawn((
                        Mesh3d(slab_mesh),
                        MeshMaterial3d(floor_material.clone()),
                        Transform::from_xyz(0.0, floor_y + 0.1, 0.0),
                    ));

                    // Columns at corners and intervals
                    let column_mesh = meshes.add(Cuboid::new(0.5, config.floor_height - 0.2, 0.5));
                    let columns_x = 3;
                    let columns_z = 2;
                    for cx in 0..=columns_x {
                        for cz in 0..=columns_z {
                            let x = -config.width / 2.0 + 1.0 + cx as f32 * (config.width - 2.0) / columns_x as f32;
                            let z = -config.depth / 2.0 + 1.0 + cz as f32 * (config.depth - 2.0) / columns_z as f32;
                            parent.spawn((
                                Mesh3d(column_mesh.clone()),
                                MeshMaterial3d(concrete_material.clone()),
                                Transform::from_xyz(x, floor_y + config.floor_height / 2.0, z),
                            ));
                        }
                    }

                    // Perimeter barriers/walls (partial height for visibility)
                    let barrier_height = 1.0;
                    let barrier_mesh_long = meshes.add(Cuboid::new(config.width, barrier_height, 0.15));
                    let barrier_mesh_short = meshes.add(Cuboid::new(0.15, barrier_height, config.depth));

                    // Front and back barriers
                    for z in [-config.depth / 2.0 + 0.1, config.depth / 2.0 - 0.1] {
                        parent.spawn((
                            Mesh3d(barrier_mesh_long.clone()),
                            MeshMaterial3d(barrier_material.clone()),
                            Transform::from_xyz(0.0, floor_y + barrier_height / 2.0 + 0.2, z),
                        ));
                    }

                    // Side barriers (with opening for ramp)
                    if floor > 0 {
                        // Left side with ramp opening
                        let side_barrier_mesh =
                            meshes.add(Cuboid::new(0.15, barrier_height, config.depth - config.ramp_width - 1.0));
                        parent.spawn((
                            Mesh3d(side_barrier_mesh.clone()),
                            MeshMaterial3d(barrier_material.clone()),
                            Transform::from_xyz(
                                -config.width / 2.0 + 0.1,
                                floor_y + barrier_height / 2.0 + 0.2,
                                (config.ramp_width + 1.0) / 2.0,
                            ),
                        ));
                    } else {
                        parent.spawn((
                            Mesh3d(barrier_mesh_short.clone()),
                            MeshMaterial3d(barrier_material.clone()),
                            Transform::from_xyz(-config.width / 2.0 + 0.1, floor_y + barrier_height / 2.0 + 0.2, 0.0),
                        ));
                    }

                    // Right side (full)
                    parent.spawn((
                        Mesh3d(barrier_mesh_short.clone()),
                        MeshMaterial3d(barrier_material.clone()),
                        Transform::from_xyz(config.width / 2.0 - 0.1, floor_y + barrier_height / 2.0 + 0.2, 0.0),
                    ));

                    // Parking space lines on floor
                    let line_mesh = meshes.add(Cuboid::new(0.1, 0.02, 2.0));
                    let spaces_per_row = 8;
                    let space_width = (config.width - 4.0) / spaces_per_row as f32;
                    for i in 0..=spaces_per_row {
                        let x = -config.width / 2.0 + 2.0 + i as f32 * space_width;
                        // Front row
                        parent.spawn((
                            Mesh3d(line_mesh.clone()),
                            MeshMaterial3d(stripe_material.clone()),
                            Transform::from_xyz(x, floor_y + 0.22, -config.depth / 4.0),
                        ));
                        // Back row
                        parent.spawn((
                            Mesh3d(line_mesh.clone()),
                            MeshMaterial3d(stripe_material.clone()),
                            Transform::from_xyz(x, floor_y + 0.22, config.depth / 4.0),
                        ));
                    }
                }

                // Ramps between floors (spiral style on one side)
                let ramp_mesh = meshes.add(Cuboid::new(config.ramp_width, 0.15, config.depth * 0.6));
                for floor in 0..floors - 1 {
                    let floor_y = floor as f32 * config.floor_height;
                    let next_floor_y = (floor + 1) as f32 * config.floor_height;
                    let ramp_y = (floor_y + next_floor_y) / 2.0;

                    // Inclined ramp
                    let ramp_angle = ((next_floor_y - floor_y) / (config.depth * 0.6)).atan();
                    parent.spawn((
                        Mesh3d(ramp_mesh.clone()),
                        MeshMaterial3d(ramp_material.clone()),
                        Transform::from_xyz(-config.width / 2.0 + config.ramp_width / 2.0 + 0.5, ramp_y, -config.depth / 4.0)
                            .with_rotation(Quat::from_rotation_x(-ramp_angle)),
                    ));
                }

                // Roof slab
                let roof_mesh = meshes.add(Cuboid::new(config.width, 0.3, config.depth));
                parent.spawn((
                    Mesh3d(roof_mesh),
                    MeshMaterial3d(concrete_material.clone()),
                    Transform::from_xyz(0.0, total_height + 0.15, 0.0),
                ));

                // Entry/exit on ground floor (opening in barrier)
                let entry_sign_mesh = meshes.add(Cuboid::new(3.0, 0.5, 0.1));
                parent.spawn((
                    Mesh3d(entry_sign_mesh),
                    MeshMaterial3d(stripe_material.clone()),
                    Transform::from_xyz(0.0, 2.5, -config.depth / 2.0 - 0.1),
                ));

                // Stairwell structure on one corner
                let stair_tower_mesh = meshes.add(Cuboid::new(3.0, total_height + 2.0, 3.0));
                parent.spawn((
                    Mesh3d(stair_tower_mesh),
                    MeshMaterial3d(concrete_material.clone()),
                    Transform::from_xyz(config.width / 2.0 - 1.5, (total_height + 2.0) / 2.0, config.depth / 2.0 - 1.5),
                ));
            });

        placed_positions.push(garage_pos);
        garage_count += 1;
    }

    info!(
        "Spawned {} parking garages with {} total capacity",
        garage_count,
        garage_count * 40 * 4
    );
}

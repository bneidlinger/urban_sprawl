//! Street furniture: fire hydrants, benches, trash cans, etc.

#![allow(dead_code)]

use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::building_factory::BuildingArchetype;
use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::building_spawner::{Building, Park, BuildingsSpawned};
use crate::render::gpu_culling::GpuCullable;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct StreetFurniturePlugin;

impl Plugin for StreetFurniturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetFurnitureConfig>()
            .init_resource::<HydrantsSpawned>()
            .init_resource::<BenchesSpawned>()
            .init_resource::<TrashCansSpawned>()
            .add_systems(Update, spawn_fire_hydrants.run_if(should_spawn_hydrants))
            .add_systems(Update, spawn_park_benches.run_if(should_spawn_benches))
            .add_systems(Update, spawn_trash_cans.run_if(should_spawn_trash_cans));
    }
}

/// Marker that hydrants have been spawned (prevents re-running).
#[derive(Resource, Default)]
pub struct HydrantsSpawned(pub bool);

/// Marker that benches have been spawned (prevents re-running).
#[derive(Resource, Default)]
pub struct BenchesSpawned(pub bool);

/// Marker that trash cans have been spawned (prevents re-running).
#[derive(Resource, Default)]
pub struct TrashCansSpawned(pub bool);

fn should_spawn_trash_cans(
    spawned: Res<BuildingsSpawned>,
    trash_spawned: Res<TrashCansSpawned>,
) -> bool {
    spawned.0 && !trash_spawned.0
}

fn should_spawn_hydrants(
    road_mesh_query: Query<&RoadMeshGenerated>,
    spawned: Res<HydrantsSpawned>,
) -> bool {
    !road_mesh_query.is_empty() && !spawned.0
}

fn should_spawn_benches(
    spawned: Res<BuildingsSpawned>,
    bench_spawned: Res<BenchesSpawned>,
) -> bool {
    spawned.0 && !bench_spawned.0
}

#[derive(Component)]
pub struct FireHydrant;

#[derive(Component)]
pub struct Bench;

#[derive(Component)]
pub struct TrashCan;

#[derive(Resource)]
pub struct StreetFurnitureConfig {
    pub hydrant_spacing: f32,
    pub hydrant_height: f32,
    pub hydrant_radius: f32,
    pub bench_length: f32,
    pub bench_height: f32,
    pub bench_depth: f32,
    pub seed: u64,
    /// Trash can height
    pub trash_can_height: f32,
    /// Trash can radius
    pub trash_can_radius: f32,
    /// Probability of trash can near commercial building
    pub trash_can_probability: f32,
}

impl Default for StreetFurnitureConfig {
    fn default() -> Self {
        Self {
            hydrant_spacing: 80.0,    // Fire hydrants every ~80m
            hydrant_height: 0.8,
            hydrant_radius: 0.15,
            bench_length: 1.8,
            bench_height: 0.45,
            bench_depth: 0.5,
            seed: 99999,
            trash_can_height: 1.0,
            trash_can_radius: 0.25,
            trash_can_probability: 0.6, // 60% of commercial buildings
        }
    }
}

fn spawn_fire_hydrants(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<StreetFurnitureConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<HydrantsSpawned>,
) {
    info!("Spawning fire hydrants...");

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Fire hydrant material (classic red/yellow)
    let hydrant_colors = [
        Color::srgb(0.7, 0.15, 0.1),  // Red
        Color::srgb(0.75, 0.65, 0.1), // Yellow
    ];

    // Hydrant meshes
    let body_mesh = meshes.add(Cylinder::new(config.hydrant_radius, config.hydrant_height));
    let cap_mesh = meshes.add(Cylinder::new(config.hydrant_radius * 1.3, 0.1));
    let nozzle_mesh = meshes.add(Cylinder::new(config.hydrant_radius * 0.5, 0.15));

    let mut hydrant_count = 0;

    for edge in road_graph.edges() {
        // Only place hydrants on major and minor roads
        let road_width = match edge.road_type {
            RoadType::Highway => continue,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => continue,
        };

        if edge.points.len() < 2 {
            continue;
        }

        // Offset to sidewalk
        let hydrant_offset = road_width / 2.0 + 3.0; // On sidewalk

        let mut accumulated_dist = rng.gen_range(0.0..config.hydrant_spacing);
        let mut segment_start_dist = 0.0;

        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let segment_length = start.distance(end);
            let segment_end_dist = segment_start_dist + segment_length;

            let dir = (end - start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);

            while accumulated_dist < segment_end_dist {
                let t = (accumulated_dist - segment_start_dist) / segment_length;
                let pos = start.lerp(end, t);

                // Alternate sides
                let side = if hydrant_count % 2 == 0 { 1.0 } else { -1.0 };
                let hydrant_pos = pos + perp * hydrant_offset * side;

                // Pick color
                let color = hydrant_colors[rng.gen_range(0..hydrant_colors.len())];
                let hydrant_material = materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.7,
                    metallic: 0.3,
                    ..default()
                });

                // Main body
                commands.spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(hydrant_material.clone()),
                    Transform::from_xyz(hydrant_pos.x, config.hydrant_height / 2.0, hydrant_pos.y),
                    FireHydrant,
                    GpuCullable::new(config.hydrant_height),
                ));

                // Cap on top
                commands.spawn((
                    Mesh3d(cap_mesh.clone()),
                    MeshMaterial3d(hydrant_material.clone()),
                    Transform::from_xyz(hydrant_pos.x, config.hydrant_height + 0.05, hydrant_pos.y),
                    FireHydrant,
                    GpuCullable::new(config.hydrant_radius * 1.3),
                ));

                // Side nozzles
                let nozzle_y = config.hydrant_height * 0.6;
                for nozzle_side in [-1.0, 1.0] {
                    let nozzle_offset = perp * config.hydrant_radius * 1.5 * nozzle_side;
                    commands.spawn((
                        Mesh3d(nozzle_mesh.clone()),
                        MeshMaterial3d(hydrant_material.clone()),
                        Transform::from_xyz(
                            hydrant_pos.x + nozzle_offset.x,
                            nozzle_y,
                            hydrant_pos.y + nozzle_offset.y,
                        )
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        FireHydrant,
                    ));
                }

                hydrant_count += 1;
                accumulated_dist += config.hydrant_spacing;
            }

            segment_start_dist = segment_end_dist;
        }
    }

    spawned.0 = true;
    info!("Spawned {} fire hydrants", hydrant_count);
}

fn spawn_park_benches(
    mut commands: Commands,
    config: Res<StreetFurnitureConfig>,
    park_query: Query<&Transform, With<Park>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BenchesSpawned>,
) {
    info!("Spawning park benches...");

    let mut rng = StdRng::seed_from_u64(config.seed + 1);

    // Bench materials
    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.2),
        perceptual_roughness: 0.85,
        ..default()
    });

    let metal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.5,
        metallic: 0.7,
        ..default()
    });

    // Bench parts
    let seat_mesh = meshes.add(Cuboid::new(config.bench_length, 0.05, config.bench_depth));
    let back_mesh = meshes.add(Cuboid::new(config.bench_length, 0.4, 0.05));
    let leg_mesh = meshes.add(Cuboid::new(0.08, config.bench_height, 0.08));

    let mut bench_count = 0;

    for park_transform in park_query.iter() {
        let park_pos = Vec2::new(park_transform.translation.x, park_transform.translation.z);

        // Place 1-3 benches per park
        let num_benches = rng.gen_range(1..=3);

        for _ in 0..num_benches {
            let offset = Vec2::new(
                rng.gen_range(-8.0..8.0),
                rng.gen_range(-8.0..8.0),
            );
            let bench_pos = park_pos + offset;

            // Random rotation
            let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

            // Seat
            commands.spawn((
                Mesh3d(seat_mesh.clone()),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(bench_pos.x, config.bench_height, bench_pos.y)
                    .with_rotation(rotation),
                Bench,
            ));

            // Back rest
            commands.spawn((
                Mesh3d(back_mesh.clone()),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(bench_pos.x, config.bench_height + 0.22, bench_pos.y)
                    .with_rotation(rotation)
                    .with_translation(
                        rotation * Vec3::new(0.0, config.bench_height + 0.22, -config.bench_depth / 2.0 + 0.025)
                            + Vec3::new(bench_pos.x, 0.0, bench_pos.y)
                    ),
                Bench,
            ));

            // Legs (4 corners)
            let leg_positions = [
                Vec3::new(config.bench_length / 2.0 - 0.1, config.bench_height / 2.0, config.bench_depth / 2.0 - 0.1),
                Vec3::new(config.bench_length / 2.0 - 0.1, config.bench_height / 2.0, -config.bench_depth / 2.0 + 0.1),
                Vec3::new(-config.bench_length / 2.0 + 0.1, config.bench_height / 2.0, config.bench_depth / 2.0 - 0.1),
                Vec3::new(-config.bench_length / 2.0 + 0.1, config.bench_height / 2.0, -config.bench_depth / 2.0 + 0.1),
            ];

            for leg_offset in leg_positions {
                let world_pos = rotation * leg_offset + Vec3::new(bench_pos.x, 0.0, bench_pos.y);
                commands.spawn((
                    Mesh3d(leg_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_translation(world_pos),
                    Bench,
                ));
            }

            bench_count += 1;
        }
    }

    spawned.0 = true;
    info!("Spawned {} park benches", bench_count);
}

/// Spawn trash cans near commercial buildings.
fn spawn_trash_cans(
    mut commands: Commands,
    config: Res<StreetFurnitureConfig>,
    building_query: Query<(&Building, &Transform), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<TrashCansSpawned>,
) {
    info!("Spawning trash cans...");

    let mut rng = StdRng::seed_from_u64(config.seed + 3);

    // Trash can materials
    let can_colors = [
        Color::srgb(0.15, 0.35, 0.15),  // Dark green (standard)
        Color::srgb(0.2, 0.2, 0.25),     // Dark gray
        Color::srgb(0.1, 0.15, 0.3),     // Dark blue
    ];

    // Trash can mesh (cylinder body)
    let body_mesh = meshes.add(Cylinder::new(config.trash_can_radius, config.trash_can_height));
    let lid_mesh = meshes.add(Cylinder::new(config.trash_can_radius * 1.1, 0.05));
    let rim_mesh = meshes.add(Torus::new(config.trash_can_radius * 0.85, config.trash_can_radius * 0.1));

    let mut trash_can_count = 0;
    let mut processed_positions: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();

    for (building, transform) in building_query.iter() {
        // Only commercial buildings get street trash cans
        if building.building_type != BuildingArchetype::Commercial {
            continue;
        }

        // Avoid placing multiple cans near the same building
        let pos_hash = (
            (transform.translation.x / 5.0) as i32,
            (transform.translation.z / 5.0) as i32,
        );
        if processed_positions.contains(&pos_hash) {
            continue;
        }
        processed_positions.insert(pos_hash);

        // Random chance
        if rng.gen::<f32>() > config.trash_can_probability {
            continue;
        }

        let building_pos = Vec2::new(transform.translation.x, transform.translation.z);

        // Place trash can in front of building (on sidewalk)
        let offset = Vec2::new(
            rng.gen_range(-3.0..3.0),
            rng.gen_range(8.0..12.0), // In front on sidewalk
        );
        let trash_pos = building_pos + offset;

        // Pick a color
        let color = can_colors[rng.gen_range(0..can_colors.len())];
        let can_material = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            metallic: 0.2,
            ..default()
        });

        // Darken lid color
        let lid_color = {
            let linear = color.to_linear();
            Color::linear_rgb(linear.red * 0.8, linear.green * 0.8, linear.blue * 0.8)
        };
        let lid_material = materials.add(StandardMaterial {
            base_color: lid_color,
            perceptual_roughness: 0.7,
            metallic: 0.3,
            ..default()
        });

        // Main body
        commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(can_material.clone()),
            Transform::from_xyz(trash_pos.x, config.trash_can_height / 2.0, trash_pos.y),
            TrashCan,
        ));

        // Lid on top
        commands.spawn((
            Mesh3d(lid_mesh.clone()),
            MeshMaterial3d(lid_material.clone()),
            Transform::from_xyz(trash_pos.x, config.trash_can_height + 0.025, trash_pos.y),
            TrashCan,
        ));

        // Rim around opening
        commands.spawn((
            Mesh3d(rim_mesh.clone()),
            MeshMaterial3d(can_material),
            Transform::from_xyz(trash_pos.x, config.trash_can_height, trash_pos.y)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            TrashCan,
        ));

        trash_can_count += 1;
    }

    spawned.0 = true;
    info!("Spawned {} trash cans near commercial buildings", trash_can_count);
}

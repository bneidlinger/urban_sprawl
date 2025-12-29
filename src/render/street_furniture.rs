//! Street furniture: fire hydrants, benches, trash cans, etc.

use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;
use crate::render::building_spawner::{Park, BuildingsSpawned};

pub struct StreetFurniturePlugin;

impl Plugin for StreetFurniturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetFurnitureConfig>()
            .add_systems(Update, spawn_fire_hydrants.run_if(should_spawn_hydrants))
            .add_systems(Update, spawn_park_benches.run_if(should_spawn_benches));
    }
}

fn should_spawn_hydrants(
    road_mesh_query: Query<&RoadMeshGenerated>,
    hydrant_query: Query<&FireHydrant>,
) -> bool {
    !road_mesh_query.is_empty() && hydrant_query.is_empty()
}

fn should_spawn_benches(
    spawned: Res<BuildingsSpawned>,
    bench_query: Query<&Bench>,
) -> bool {
    spawned.0 && bench_query.is_empty()
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
        }
    }
}

fn spawn_fire_hydrants(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<StreetFurnitureConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
                ));

                // Cap on top
                commands.spawn((
                    Mesh3d(cap_mesh.clone()),
                    MeshMaterial3d(hydrant_material.clone()),
                    Transform::from_xyz(hydrant_pos.x, config.hydrant_height + 0.05, hydrant_pos.y),
                    FireHydrant,
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

    info!("Spawned {} fire hydrants", hydrant_count);
}

fn spawn_park_benches(
    mut commands: Commands,
    config: Res<StreetFurnitureConfig>,
    park_query: Query<&Transform, With<Park>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    info!("Spawned {} park benches", bench_count);
}

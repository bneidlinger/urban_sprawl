//! Parked car generation along roads.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::gpu_culling::GpuCullable;
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct ParkedCarsPlugin;

impl Plugin for ParkedCarsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParkedCarConfig>()
            .init_resource::<ParkedCarsSpawned>()
            .add_systems(Update, spawn_parked_cars.run_if(should_spawn_cars));
    }
}

/// Marker that parked cars have been spawned (prevents re-running).
#[derive(Resource, Default)]
pub struct ParkedCarsSpawned(pub bool);

fn should_spawn_cars(
    road_mesh_query: Query<&RoadMeshGenerated>,
    spawned: Res<ParkedCarsSpawned>,
) -> bool {
    !road_mesh_query.is_empty() && !spawned.0
}

#[derive(Component)]
pub struct ParkedCar;

#[derive(Resource)]
pub struct ParkedCarConfig {
    pub car_length: f32,
    pub car_width: f32,
    pub car_height: f32,
    pub wheel_radius: f32,
    pub parking_probability: f32,
    pub min_spacing: f32,
    pub offset_from_road_edge: f32,
    pub seed: u64,
}

impl Default for ParkedCarConfig {
    fn default() -> Self {
        Self {
            car_length: 4.5,          // Typical sedan length
            car_width: 1.8,           // Typical car width
            car_height: 1.4,          // Sedan height
            wheel_radius: 0.35,
            parking_probability: 0.4, // 40% chance of parking spot being filled
            min_spacing: 6.0,         // Space between potential parking spots
            offset_from_road_edge: 1.5,
            seed: 12345,
        }
    }
}

// Car color palette
const CAR_COLORS: &[(f32, f32, f32)] = &[
    (0.1, 0.1, 0.12),   // Black
    (0.9, 0.9, 0.92),   // White
    (0.6, 0.6, 0.65),   // Silver
    (0.15, 0.15, 0.2),  // Dark gray
    (0.5, 0.1, 0.1),    // Dark red
    (0.1, 0.2, 0.4),    // Dark blue
    (0.2, 0.25, 0.2),   // Dark green
    (0.4, 0.35, 0.25),  // Brown/tan
];

fn spawn_parked_cars(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<ParkedCarConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut spawned: ResMut<ParkedCarsSpawned>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning parked cars...");

    // Create terrain sampler
    let terrain = TerrainSampler::new(&terrain_config);

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Pre-create car body mesh
    let body_mesh = meshes.add(Cuboid::new(config.car_length, config.car_height * 0.6, config.car_width));
    let cabin_mesh = meshes.add(Cuboid::new(config.car_length * 0.5, config.car_height * 0.4, config.car_width * 0.9));
    let wheel_mesh = meshes.add(Cylinder::new(config.wheel_radius, 0.2));

    // Wheel material (dark rubber)
    let wheel_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Window material (tinted glass)
    let window_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.15, 0.2, 0.8),
        perceptual_roughness: 0.1,
        metallic: 0.3,
        ..default()
    });

    let mut car_count = 0;

    for edge in road_graph.edges() {
        // Only park on major and minor roads (not highways or alleys)
        let road_width = match edge.road_type {
            RoadType::Highway => continue,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => continue,
        };

        if edge.points.len() < 2 {
            continue;
        }

        // Calculate parking offset from road center
        let parking_offset = road_width / 2.0 + config.offset_from_road_edge;

        // Walk along the road and potentially place parked cars
        let mut accumulated_dist = config.min_spacing;
        let mut segment_start_dist = 0.0;

        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let segment_length = start.distance(end);
            let segment_end_dist = segment_start_dist + segment_length;

            let dir = (end - start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);

            while accumulated_dist < segment_end_dist {
                // Random chance to spawn a car
                if rng.gen::<f32>() < config.parking_probability {
                    let t = (accumulated_dist - segment_start_dist) / segment_length;
                    let pos = start.lerp(end, t);

                    // Alternate sides, with some randomness
                    let side = if rng.gen::<bool>() { 1.0 } else { -1.0 };
                    let car_pos = pos + perp * parking_offset * side;

                    // Random car color
                    let (r, g, b) = CAR_COLORS[rng.gen_range(0..CAR_COLORS.len())];
                    let car_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(r, g, b),
                        perceptual_roughness: 0.4,
                        metallic: 0.6,
                        ..default()
                    });

                    // Calculate rotation to align with road
                    let angle = dir.y.atan2(dir.x);
                    // Add slight random angle variation for realism
                    let angle_variation = rng.gen_range(-0.05..0.05);
                    let rotation = Quat::from_rotation_y(-angle + angle_variation);

                    // Sample terrain height at car position
                    let terrain_height = terrain.sample(car_pos.x, car_pos.y);
                    let body_y = terrain_height + config.wheel_radius + config.car_height * 0.3;

                    // Car bounding radius (diagonal of body)
                    let car_radius = (config.car_length * config.car_length + config.car_width * config.car_width + config.car_height * config.car_height).sqrt() / 2.0;

                    // Spawn car body
                    commands.spawn((
                        Mesh3d(body_mesh.clone()),
                        MeshMaterial3d(car_material.clone()),
                        Transform::from_xyz(car_pos.x, body_y, car_pos.y)
                            .with_rotation(rotation),
                        ParkedCar,
                        GpuCullable::new(car_radius),
                    ));

                    // Spawn cabin (on top of body)
                    let cabin_y = body_y + config.car_height * 0.5;
                    commands.spawn((
                        Mesh3d(cabin_mesh.clone()),
                        MeshMaterial3d(window_material.clone()),
                        Transform::from_xyz(car_pos.x, cabin_y, car_pos.y)
                            .with_rotation(rotation),
                        ParkedCar,
                        GpuCullable::new(car_radius * 0.5),
                    ));

                    // Spawn wheels (4 corners)
                    let wheel_positions = [
                        Vec2::new(config.car_length * 0.35, config.car_width * 0.45),
                        Vec2::new(config.car_length * 0.35, -config.car_width * 0.45),
                        Vec2::new(-config.car_length * 0.35, config.car_width * 0.45),
                        Vec2::new(-config.car_length * 0.35, -config.car_width * 0.45),
                    ];

                    for wheel_offset in wheel_positions {
                        // Rotate wheel offset by car angle
                        let rotated_offset = Vec2::new(
                            wheel_offset.x * angle.cos() + wheel_offset.y * angle.sin(),
                            -wheel_offset.x * angle.sin() + wheel_offset.y * angle.cos(),
                        );

                        let wheel_pos = car_pos + rotated_offset;
                        let wheel_terrain = terrain.sample(wheel_pos.x, wheel_pos.y);

                        commands.spawn((
                            Mesh3d(wheel_mesh.clone()),
                            MeshMaterial3d(wheel_material.clone()),
                            Transform::from_xyz(wheel_pos.x, wheel_terrain + config.wheel_radius, wheel_pos.y)
                                .with_rotation(rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                            ParkedCar,
                            GpuCullable::new(config.wheel_radius),
                        ));
                    }

                    car_count += 1;
                }

                accumulated_dist += config.min_spacing;
            }

            segment_start_dist = segment_end_dist;
        }
    }

    spawned.0 = true;
    info!("Spawned {} parked cars", car_count);
}

/// Helper struct for sampling terrain height.
struct TerrainSampler {
    perlin: Perlin,
    noise_scale: f32,
    height_scale: f32,
    octaves: u32,
}

impl TerrainSampler {
    fn new(config: &TerrainConfig) -> Self {
        Self {
            perlin: Perlin::new(config.seed),
            noise_scale: config.noise_scale,
            height_scale: config.height_scale,
            octaves: config.octaves,
        }
    }

    fn sample(&self, x: f32, z: f32) -> f32 {
        let mut height = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.noise_scale;
        let mut max_amplitude = 0.0;

        for _ in 0..self.octaves {
            let sample_x = x as f64 * frequency as f64;
            let sample_z = z as f64 * frequency as f64;
            height += self.perlin.get([sample_x, sample_z]) as f32 * amplitude;
            max_amplitude += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        (height / max_amplitude) * self.height_scale
    }
}

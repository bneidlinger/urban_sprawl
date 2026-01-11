//! Parked car generation along roads.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::gpu_culling::GpuCullable;
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;
use crate::render::vehicle_meshes::{generate_vehicle_mesh, generate_wheel_mesh, VehicleMeshConfig, VehicleShape};

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

/// Parked vehicle types with their shape configurations.
#[derive(Clone, Copy)]
enum ParkedVehicleType {
    Sedan,
    SUV,
    Hatchback,
    SportsCar,
    Van,
    Truck,
}

impl ParkedVehicleType {
    fn random(rng: &mut StdRng) -> Self {
        match rng.gen_range(0..100) {
            0..=35 => ParkedVehicleType::Sedan,
            36..=55 => ParkedVehicleType::SUV,
            56..=70 => ParkedVehicleType::Hatchback,
            71..=80 => ParkedVehicleType::SportsCar,
            81..=90 => ParkedVehicleType::Van,
            _ => ParkedVehicleType::Truck,
        }
    }

    fn to_config(&self) -> VehicleMeshConfig {
        match self {
            ParkedVehicleType::Sedan => VehicleMeshConfig {
                length: 4.5,
                width: 1.8,
                height: 1.4,
                shape: VehicleShape::Sedan,
            },
            ParkedVehicleType::SUV => VehicleMeshConfig {
                length: 4.8,
                width: 1.9,
                height: 1.7,
                shape: VehicleShape::SUV,
            },
            ParkedVehicleType::Hatchback => VehicleMeshConfig {
                length: 4.0,
                width: 1.75,
                height: 1.45,
                shape: VehicleShape::Hatchback,
            },
            ParkedVehicleType::SportsCar => VehicleMeshConfig {
                length: 4.3,
                width: 1.85,
                height: 1.25,
                shape: VehicleShape::SportsCar,
            },
            ParkedVehicleType::Van => VehicleMeshConfig {
                length: 5.0,
                width: 1.9,
                height: 2.0,
                shape: VehicleShape::Van,
            },
            ParkedVehicleType::Truck => VehicleMeshConfig {
                length: 5.5,
                width: 2.0,
                height: 1.9,
                shape: VehicleShape::Truck,
            },
        }
    }

    fn wheel_radius(&self) -> f32 {
        match self {
            ParkedVehicleType::Sedan | ParkedVehicleType::Hatchback => 0.38,
            ParkedVehicleType::SUV | ParkedVehicleType::Truck => 0.45,
            ParkedVehicleType::SportsCar => 0.35,
            ParkedVehicleType::Van => 0.40,
        }
    }
}

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

    // Pre-generate vehicle meshes for each type
    let sedan_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::Sedan.to_config()));
    let suv_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::SUV.to_config()));
    let hatchback_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::Hatchback.to_config()));
    let sports_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::SportsCar.to_config()));
    let van_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::Van.to_config()));
    let truck_mesh = meshes.add(generate_vehicle_mesh(&ParkedVehicleType::Truck.to_config()));

    // Generate wheel meshes for different sizes (larger, more visible)
    let wheel_small = meshes.add(generate_wheel_mesh(0.35, 0.26));
    let wheel_medium = meshes.add(generate_wheel_mesh(0.40, 0.28));
    let wheel_large = meshes.add(generate_wheel_mesh(0.45, 0.32));

    // Wheel material (dark rubber with subtle metallic hub)
    let wheel_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.12),
        perceptual_roughness: 0.85,
        metallic: 0.1,
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

                    // Pick random vehicle type
                    let vehicle_type = ParkedVehicleType::random(&mut rng);
                    let vehicle_config = vehicle_type.to_config();
                    let wheel_radius = vehicle_type.wheel_radius();

                    // Select mesh based on vehicle type
                    let body_mesh_handle = match vehicle_type {
                        ParkedVehicleType::Sedan => sedan_mesh.clone(),
                        ParkedVehicleType::SUV => suv_mesh.clone(),
                        ParkedVehicleType::Hatchback => hatchback_mesh.clone(),
                        ParkedVehicleType::SportsCar => sports_mesh.clone(),
                        ParkedVehicleType::Van => van_mesh.clone(),
                        ParkedVehicleType::Truck => truck_mesh.clone(),
                    };

                    // Select wheel mesh based on size
                    let wheel_mesh_handle = if wheel_radius <= 0.37 {
                        wheel_small.clone()
                    } else if wheel_radius <= 0.42 {
                        wheel_medium.clone()
                    } else {
                        wheel_large.clone()
                    };

                    // Random car color - car paint is NOT metallic, it's clear coat over pigment
                    let (r, g, b) = CAR_COLORS[rng.gen_range(0..CAR_COLORS.len())];
                    let car_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(r, g, b),
                        perceptual_roughness: 0.55, // Moderate shine, not mirror-like
                        metallic: 0.0,              // Car paint is dielectric, not metallic
                        reflectance: 0.35,          // Standard clear coat reflectance
                        ..default()
                    });

                    // Calculate rotation to align hood with direction of travel
                    let base_yaw = (-dir.x).atan2(-dir.y);
                    // Add slight random angle variation for realism
                    let angle_variation = rng.gen_range(-0.05..0.05);
                    let rotation = Quat::from_rotation_y(base_yaw + angle_variation);

                    // Spawn wheels (4 corners)
                    let wheel_positions = [
                        Vec2::new(vehicle_config.length * 0.35, vehicle_config.width * 0.42),
                        Vec2::new(vehicle_config.length * 0.35, -vehicle_config.width * 0.42),
                        Vec2::new(-vehicle_config.length * 0.32, vehicle_config.width * 0.42),
                        Vec2::new(-vehicle_config.length * 0.32, -vehicle_config.width * 0.42),
                    ];

                    let mut max_wheel_surface = f32::MIN;

                    for wheel_offset in wheel_positions {
                        let wheel_world_pos = car_pos + dir * wheel_offset.x + perp * wheel_offset.y;
                        let wheel_terrain = terrain.sample(wheel_world_pos.x, wheel_world_pos.y);
                        let wheel_road_surface = wheel_terrain + 0.12; // Road height offset
                        max_wheel_surface = max_wheel_surface.max(wheel_road_surface);

                        // Wheel rotation: the wheel mesh has its axle along Z, we need it along the car's width (local X)
                        // Rotate 90 degrees around Y to reorient the axle, then apply car rotation
                        let wheel_rotation = rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

                        commands.spawn((
                            Mesh3d(wheel_mesh_handle.clone()),
                            MeshMaterial3d(wheel_material.clone()),
                            Transform::from_xyz(wheel_world_pos.x, wheel_road_surface + wheel_radius, wheel_world_pos.y)
                                .with_rotation(wheel_rotation),
                            ParkedCar,
                            GpuCullable::new(wheel_radius),
                        ));
                    }

                    // Road surface is above terrain, and body sits on the highest wheel
                    let body_y = max_wheel_surface + wheel_radius * 0.6;

                    // Car bounding radius
                    let car_radius = (vehicle_config.length * vehicle_config.length
                        + vehicle_config.width * vehicle_config.width
                        + vehicle_config.height * vehicle_config.height).sqrt() / 2.0;

                    // Spawn car body (combined body + cabin mesh)
                    commands.spawn((
                        Mesh3d(body_mesh_handle),
                        MeshMaterial3d(car_material.clone()),
                        Transform::from_xyz(car_pos.x, body_y, car_pos.y)
                            .with_rotation(rotation),
                        ParkedCar,
                        GpuCullable::new(car_radius),
                    ));

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

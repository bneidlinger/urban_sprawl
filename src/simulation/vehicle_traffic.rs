//! Moving vehicle traffic system.
//!
//! Spawns vehicles that drive along the road network, following waypoints
//! and stopping at intersections.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use petgraph::graph::{EdgeIndex, NodeIndex};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadNodeType, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;
use crate::render::traffic_lights::{LightPhase, TrafficLightController};
use crate::simulation::vehicles::{MovingVehicle, VehicleNavigation};

pub struct MovingVehiclePlugin;

impl Plugin for MovingVehiclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovingVehicleConfig>()
            .init_resource::<VehiclesInitialized>()
            .add_systems(
                Update,
                (
                    spawn_moving_vehicles.run_if(should_spawn_vehicles),
                    vehicle_traffic_light_check,
                    vehicle_movement,
                    vehicle_edge_transition,
                    vehicle_transform_sync,
                )
                    .chain(),
            );
    }
}

/// Configuration for moving vehicles.
#[derive(Resource)]
pub struct MovingVehicleConfig {
    pub target_count: usize,
    pub base_speed: f32,
    pub speed_variation: f32,
    pub car_length: f32,
    pub car_width: f32,
    pub car_height: f32,
    pub seed: u64,
}

impl Default for MovingVehicleConfig {
    fn default() -> Self {
        Self {
            target_count: 25,
            base_speed: 12.0,       // ~43 km/h
            speed_variation: 0.15,  // +/- 15%
            car_length: 4.2,
            car_width: 1.7,
            car_height: 1.3,
            seed: 99999,
        }
    }
}

/// Marker that vehicle spawning has been initialized.
#[derive(Resource, Default)]
pub struct VehiclesInitialized(pub bool);

/// Run condition: spawn vehicles when roads exist and we haven't reached target count.
fn should_spawn_vehicles(
    road_mesh_query: Query<&RoadMeshGenerated>,
    vehicle_query: Query<&MovingVehicle>,
    config: Res<MovingVehicleConfig>,
    initialized: Res<VehiclesInitialized>,
) -> bool {
    !road_mesh_query.is_empty()
        && (vehicle_query.iter().count() < config.target_count || !initialized.0)
}

// Car color palette (same as parked cars for consistency)
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

fn spawn_moving_vehicles(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<MovingVehicleConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle_query: Query<&MovingVehicle>,
    mut initialized: ResMut<VehiclesInitialized>,
    mut local_rng: Local<Option<StdRng>>,
) {
    // Initialize RNG on first run
    let rng = local_rng.get_or_insert_with(|| StdRng::seed_from_u64(config.seed));

    let current_count = vehicle_query.iter().count();
    if current_count >= config.target_count {
        initialized.0 = true;
        return;
    }

    // Collect valid intersection nodes (nodes with 2+ neighbors)
    let intersections: Vec<NodeIndex> = road_graph
        .nodes()
        .filter_map(|(idx, node)| {
            let neighbor_count = road_graph.neighbors(idx).count();
            if neighbor_count >= 2 && node.node_type == RoadNodeType::Intersection {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    if intersections.is_empty() {
        warn!("No valid intersections found for vehicle spawning");
        initialized.0 = true;
        return;
    }

    // Create meshes
    let body_mesh = meshes.add(Cuboid::new(
        config.car_length,
        config.car_height * 0.6,
        config.car_width,
    ));
    let cabin_mesh = meshes.add(Cuboid::new(
        config.car_length * 0.5,
        config.car_height * 0.4,
        config.car_width * 0.9,
    ));

    // Window material
    let window_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.15, 0.2, 0.8),
        perceptual_roughness: 0.1,
        metallic: 0.3,
        ..default()
    });

    let terrain = TerrainSampler::new(&terrain_config);

    // Spawn vehicles up to target count
    let to_spawn = (config.target_count - current_count).min(5); // Spawn max 5 per frame

    for _ in 0..to_spawn {
        // Pick random intersection
        let start_node_idx = intersections[rng.gen_range(0..intersections.len())];

        // Get edges from this node
        let edges: Vec<EdgeIndex> = road_graph.edges_of_node(start_node_idx).collect();
        if edges.is_empty() {
            continue;
        }

        // Pick random edge
        let edge_idx = edges[rng.gen_range(0..edges.len())];
        let Some((node_a, node_b)) = road_graph.edge_endpoints(edge_idx) else {
            continue;
        };

        // Determine direction (forward = heading away from start_node)
        let (forward, dest_node) = if node_a == start_node_idx {
            (true, node_b)
        } else {
            (false, node_a)
        };

        // Random speed with variation
        let speed_mult = 1.0 + rng.gen_range(-config.speed_variation..config.speed_variation);
        let target_speed = config.base_speed * speed_mult;

        // Get road type to adjust speed
        let edge = road_graph.edge_by_index(edge_idx).unwrap();
        let road_speed_mult = match edge.road_type {
            RoadType::Highway => 1.5,
            RoadType::Major => 1.0,
            RoadType::Minor => 0.8,
            RoadType::Alley => 0.5,
        };
        let final_speed = target_speed * road_speed_mult;

        // Random car color
        let (r, g, b) = CAR_COLORS[rng.gen_range(0..CAR_COLORS.len())];
        let car_material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.4,
            metallic: 0.6,
            ..default()
        });

        // Get initial position
        let points = &edge.points;
        let (pos, dir) = if forward {
            interpolate_edge_position(points, 0.0)
        } else {
            let (p, d) = interpolate_edge_position(points, 1.0);
            (p, -d)
        };

        let terrain_height = terrain.sample(pos.x, pos.y);
        let body_y = terrain_height + config.car_height * 0.5;
        let angle = dir.y.atan2(dir.x);
        let rotation = Quat::from_rotation_y(-angle);

        // Spawn car body with navigation
        commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(car_material.clone()),
            Transform::from_xyz(pos.x, body_y, pos.y).with_rotation(rotation),
            MovingVehicle,
            VehicleNavigation {
                current_edge: edge_idx,
                forward,
                progress: if forward { 0.0 } else { 1.0 },
                speed: final_speed,
                target_speed: final_speed,
                destination_node: dest_node,
                previous_node: Some(start_node_idx),
                stopping: false,
            },
        ));

        // Spawn cabin as child-like entity at same position
        // (We'll update its position in sync system)
        let cabin_y = body_y + config.car_height * 0.5;
        commands.spawn((
            Mesh3d(cabin_mesh.clone()),
            MeshMaterial3d(window_material.clone()),
            Transform::from_xyz(pos.x, cabin_y, pos.y).with_rotation(rotation),
            MovingVehicle,
            // No navigation - this is just visual, synced with body
        ));
    }

    if current_count + to_spawn >= config.target_count {
        initialized.0 = true;
        info!("Spawned {} moving vehicles", config.target_count);
    }
}

/// Check for traffic lights at upcoming intersections and stop if red.
fn vehicle_traffic_light_check(
    mut vehicles: Query<&mut VehicleNavigation, With<MovingVehicle>>,
    traffic_lights: Query<&TrafficLightController>,
) {
    // Build a quick lookup of node -> light phase
    let mut light_phases: std::collections::HashMap<NodeIndex, LightPhase> =
        std::collections::HashMap::new();

    for controller in traffic_lights.iter() {
        light_phases.insert(controller.node_index, controller.phase);
    }

    for mut nav in vehicles.iter_mut() {
        // Check if approaching the end of the edge (progress > 0.7)
        let approaching_end = if nav.forward {
            nav.progress > 0.7
        } else {
            nav.progress < 0.3
        };

        if !approaching_end {
            // Not near intersection, don't stop
            nav.stopping = false;
            continue;
        }

        // Check if there's a traffic light at our destination node
        if let Some(&phase) = light_phases.get(&nav.destination_node) {
            // Stop for red or yellow lights
            nav.stopping = matches!(phase, LightPhase::Red | LightPhase::Yellow);
        } else {
            // No traffic light at this intersection
            nav.stopping = false;
        }
    }
}

/// Advance vehicle progress along their current edge.
fn vehicle_movement(
    time: Res<Time>,
    road_graph: Res<RoadGraph>,
    mut vehicles: Query<&mut VehicleNavigation, With<MovingVehicle>>,
) {
    let dt = time.delta_secs();

    for mut nav in vehicles.iter_mut() {
        if nav.stopping {
            // Decelerate when stopping
            nav.speed = (nav.speed - 8.0 * dt).max(0.0);
        } else {
            // Accelerate toward target speed
            nav.speed = (nav.speed + 3.0 * dt).min(nav.target_speed);
        }

        if nav.speed <= 0.001 {
            continue;
        }

        // Get edge length
        let Some(edge) = road_graph.edge_by_index(nav.current_edge) else {
            continue;
        };

        let edge_length = edge.length;
        if edge_length <= 0.0 {
            continue;
        }

        // Calculate progress delta
        let distance = nav.speed * dt;
        let progress_delta = distance / edge_length;

        // Update progress based on direction
        if nav.forward {
            nav.progress += progress_delta;
        } else {
            nav.progress -= progress_delta;
        }
    }
}

/// Handle vehicles reaching the end of their current edge.
fn vehicle_edge_transition(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    mut vehicles: Query<(Entity, &mut VehicleNavigation), With<MovingVehicle>>,
    mut local_rng: Local<Option<StdRng>>,
) {
    let rng = local_rng.get_or_insert_with(|| StdRng::seed_from_u64(77777));

    for (entity, mut nav) in vehicles.iter_mut() {
        // Check if we've reached the end of the edge
        let at_end = (nav.forward && nav.progress >= 1.0) || (!nav.forward && nav.progress <= 0.0);

        if !at_end {
            continue;
        }

        // We've reached the destination node
        let current_node = nav.destination_node;

        // Get all edges from this node
        let edges: Vec<EdgeIndex> = road_graph.edges_of_node(current_node).collect();

        // Filter out the edge we came from (to avoid immediate U-turn)
        let valid_edges: Vec<EdgeIndex> = edges
            .into_iter()
            .filter(|&e| e != nav.current_edge)
            .collect();

        if valid_edges.is_empty() {
            // Dead end - despawn this vehicle (it will be respawned elsewhere)
            commands.entity(entity).despawn();
            continue;
        }

        // Pick random next edge
        let next_edge = valid_edges[rng.gen_range(0..valid_edges.len())];
        let Some((node_a, node_b)) = road_graph.edge_endpoints(next_edge) else {
            commands.entity(entity).despawn();
            continue;
        };

        // Determine direction on new edge
        let (forward, dest_node) = if node_a == current_node {
            (true, node_b)
        } else {
            (false, node_a)
        };

        // Update speed based on road type
        if let Some(edge) = road_graph.edge_by_index(next_edge) {
            let road_speed_mult = match edge.road_type {
                RoadType::Highway => 1.5,
                RoadType::Major => 1.0,
                RoadType::Minor => 0.8,
                RoadType::Alley => 0.5,
            };
            nav.target_speed = 12.0 * road_speed_mult; // Base speed * road mult
        }

        // Update navigation state
        nav.previous_node = Some(current_node);
        nav.current_edge = next_edge;
        nav.forward = forward;
        nav.progress = if forward { 0.0 } else { 1.0 };
        nav.destination_node = dest_node;
        nav.stopping = false;
    }
}

/// Update vehicle transforms based on their navigation state.
fn vehicle_transform_sync(
    road_graph: Res<RoadGraph>,
    terrain_config: Res<TerrainConfig>,
    config: Res<MovingVehicleConfig>,
    mut vehicles: Query<(&VehicleNavigation, &mut Transform), With<MovingVehicle>>,
) {
    let terrain = TerrainSampler::new(&terrain_config);

    for (nav, mut transform) in vehicles.iter_mut() {
        let Some(edge) = road_graph.edge_by_index(nav.current_edge) else {
            continue;
        };

        // Clamp progress to valid range
        let progress = nav.progress.clamp(0.0, 1.0);

        // Get position and direction along edge
        let (pos, mut dir) = interpolate_edge_position(&edge.points, progress);

        // Flip direction if traveling backward
        if !nav.forward {
            dir = -dir;
        }

        // Update position with terrain height
        let terrain_height = terrain.sample(pos.x, pos.y);
        let body_y = terrain_height + config.car_height * 0.5;

        transform.translation.x = pos.x;
        transform.translation.y = body_y;
        transform.translation.z = pos.y;

        // Update rotation to face direction of travel
        if dir.length_squared() > 0.001 {
            let angle = dir.y.atan2(dir.x);
            transform.rotation = Quat::from_rotation_y(-angle);
        }
    }
}

/// Interpolate position and direction along edge waypoints.
fn interpolate_edge_position(points: &[Vec2], progress: f32) -> (Vec2, Vec2) {
    if points.is_empty() {
        return (Vec2::ZERO, Vec2::X);
    }
    if points.len() == 1 {
        return (points[0], Vec2::X);
    }

    // Calculate total length
    let total_length: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();

    if total_length <= 0.0 {
        return (points[0], Vec2::X);
    }

    let target_dist = progress.clamp(0.0, 1.0) * total_length;
    let mut accumulated = 0.0;

    for window in points.windows(2) {
        let seg_len = window[0].distance(window[1]);
        if accumulated + seg_len >= target_dist {
            let local_t = if seg_len > 0.0 {
                (target_dist - accumulated) / seg_len
            } else {
                0.0
            };
            let pos = window[0].lerp(window[1], local_t);
            let dir = (window[1] - window[0]).normalize_or_zero();
            return (pos, dir);
        }
        accumulated += seg_len;
    }

    // Return end point
    let last = *points.last().unwrap();
    let dir = if points.len() >= 2 {
        (points[points.len() - 1] - points[points.len() - 2]).normalize_or_zero()
    } else {
        Vec2::X
    };
    (last, dir)
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

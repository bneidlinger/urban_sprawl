//! Moving vehicle traffic system.
//!
//! Spawns vehicles that drive along the road network, following waypoints
//! and stopping at intersections. Supports multiple vehicle types including
//! sedans, SUVs, trucks, vans, and buses.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use petgraph::graph::{EdgeIndex, NodeIndex};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadNodeType, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;
use crate::render::traffic_lights::{LightPhase, TrafficLightController};
use crate::render::vehicle_meshes::{generate_vehicle_mesh, generate_wheel_mesh, VehicleMeshConfig, VehicleShape};
use crate::simulation::vehicles::{MovingVehicle, VehicleNavigation};

/// Different types of vehicles with varying sizes and speeds.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleType {
    Sedan,
    SUV,
    Truck,
    Van,
    Bus,
    // Emergency vehicles
    PoliceCar,
    FireTruck,
    Ambulance,
}

impl VehicleType {
    /// Get vehicle dimensions (length, width, height).
    pub fn dimensions(&self) -> (f32, f32, f32) {
        match self {
            VehicleType::Sedan => (4.2, 1.7, 1.3),
            VehicleType::SUV => (4.8, 1.9, 1.7),
            VehicleType::Truck => (5.5, 2.0, 2.2),
            VehicleType::Van => (5.0, 1.9, 2.0),
            VehicleType::Bus => (12.0, 2.5, 3.2),
            VehicleType::PoliceCar => (4.5, 1.8, 1.4),
            VehicleType::FireTruck => (9.0, 2.4, 3.0),
            VehicleType::Ambulance => (6.0, 2.1, 2.4),
        }
    }

    /// Get cabin dimensions relative to body.
    pub fn cabin_dimensions(&self) -> (f32, f32, f32) {
        let (length, width, height) = self.dimensions();
        match self {
            VehicleType::Sedan => (length * 0.5, width * 0.9, height * 0.4),
            VehicleType::SUV => (length * 0.6, width * 0.9, height * 0.5),
            VehicleType::Truck => (length * 0.35, width * 0.9, height * 0.4),
            VehicleType::Van => (length * 0.7, width * 0.9, height * 0.6),
            VehicleType::Bus => (length * 0.85, width * 0.95, height * 0.7),
            VehicleType::PoliceCar => (length * 0.5, width * 0.9, height * 0.4),
            VehicleType::FireTruck => (length * 0.3, width * 0.9, height * 0.45),
            VehicleType::Ambulance => (length * 0.7, width * 0.95, height * 0.7),
        }
    }

    /// Get cabin offset from center (trucks/fire trucks have cab at front).
    pub fn cabin_offset(&self) -> f32 {
        let (length, _, _) = self.dimensions();
        match self {
            VehicleType::Truck => length * 0.25,
            VehicleType::FireTruck => length * 0.3,
            _ => 0.0,
        }
    }

    /// Get speed multiplier for this vehicle type.
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            VehicleType::Sedan => 1.0,
            VehicleType::SUV => 0.95,
            VehicleType::Truck => 0.75,
            VehicleType::Van => 0.85,
            VehicleType::Bus => 0.7,
            // Emergency vehicles go faster!
            VehicleType::PoliceCar => 1.3,
            VehicleType::FireTruck => 1.1,
            VehicleType::Ambulance => 1.2,
        }
    }

    /// Get spawn weight (relative probability).
    pub fn spawn_weight(&self) -> f32 {
        match self {
            VehicleType::Sedan => 40.0,
            VehicleType::SUV => 25.0,
            VehicleType::Truck => 10.0,
            VehicleType::Van => 15.0,
            VehicleType::Bus => 10.0,
            // Emergency vehicles are rare
            VehicleType::PoliceCar => 2.0,
            VehicleType::FireTruck => 1.0,
            VehicleType::Ambulance => 1.5,
        }
    }

    /// Returns true if this is an emergency vehicle.
    pub fn is_emergency(&self) -> bool {
        matches!(self, VehicleType::PoliceCar | VehicleType::FireTruck | VehicleType::Ambulance)
    }

    /// Get the VehicleShape for mesh generation.
    pub fn vehicle_shape(&self) -> VehicleShape {
        match self {
            VehicleType::Sedan | VehicleType::PoliceCar => VehicleShape::Sedan,
            VehicleType::SUV => VehicleShape::SUV,
            VehicleType::Truck | VehicleType::FireTruck => VehicleShape::Truck,
            VehicleType::Van | VehicleType::Ambulance => VehicleShape::Van,
            VehicleType::Bus => VehicleShape::Bus,
        }
    }

    /// Get wheel radius for this vehicle type (larger for visibility).
    pub fn wheel_radius(&self) -> f32 {
        match self {
            VehicleType::Sedan | VehicleType::PoliceCar => 0.38,
            VehicleType::SUV => 0.44,
            VehicleType::Truck | VehicleType::FireTruck => 0.48,
            VehicleType::Van | VehicleType::Ambulance => 0.42,
            VehicleType::Bus => 0.50,
        }
    }

    /// Get mesh configuration for this vehicle type.
    pub fn mesh_config(&self) -> VehicleMeshConfig {
        let (length, width, height) = self.dimensions();
        VehicleMeshConfig {
            length,
            width,
            height,
            shape: self.vehicle_shape(),
        }
    }

    /// Get the body color for this vehicle type.
    pub fn body_color(&self, rng: &mut StdRng) -> (f32, f32, f32) {
        match self {
            VehicleType::PoliceCar => {
                // Black and white police cars
                if rng.gen_bool(0.5) {
                    (0.1, 0.1, 0.12) // Black
                } else {
                    (0.95, 0.95, 0.95) // White
                }
            }
            VehicleType::FireTruck => (0.8, 0.15, 0.1), // Fire engine red
            VehicleType::Ambulance => (0.95, 0.95, 0.95), // White
            _ => CAR_COLORS[rng.gen_range(0..CAR_COLORS.len())],
        }
    }

    /// Pick a random vehicle type based on spawn weights.
    pub fn random(rng: &mut StdRng) -> Self {
        let all_types = [
            VehicleType::Sedan,
            VehicleType::SUV,
            VehicleType::Truck,
            VehicleType::Van,
            VehicleType::Bus,
            VehicleType::PoliceCar,
            VehicleType::FireTruck,
            VehicleType::Ambulance,
        ];

        let total_weight: f32 = all_types.iter().map(|v| v.spawn_weight()).sum();

        let mut roll = rng.gen::<f32>() * total_weight;
        for vtype in all_types {
            roll -= vtype.spawn_weight();
            if roll <= 0.0 {
                return vtype;
            }
        }
        VehicleType::Sedan
    }
}

/// Component for emergency vehicle siren state.
#[derive(Component)]
pub struct EmergencySiren {
    /// Current siren phase (0.0 to 1.0, wraps around)
    pub phase: f32,
    /// Siren cycle speed (cycles per second)
    pub frequency: f32,
    /// Whether siren is currently active
    pub active: bool,
}

impl Default for EmergencySiren {
    fn default() -> Self {
        Self {
            phase: 0.0,
            frequency: 2.0, // 2 cycles per second
            active: true,
        }
    }
}

/// Marker for emergency light entities.
#[derive(Component)]
pub struct EmergencyLight {
    /// Parent vehicle entity
    pub vehicle: Entity,
    /// Which light bar position (0 = left, 1 = right)
    pub position: u8,
    /// Base color (red or blue)
    pub color: EmergencyLightColor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmergencyLightColor {
    Red,
    Blue,
}

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
                    vehicle_lane_change,
                    vehicle_transform_sync,
                    update_emergency_sirens,
                    update_emergency_lights,
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
    pub seed: u64,
}

impl Default for MovingVehicleConfig {
    fn default() -> Self {
        Self {
            target_count: 25,
            base_speed: 12.0,       // ~43 km/h
            speed_variation: 0.15,  // +/- 15%
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

    // Bus-specific yellow/orange color
    let bus_colors: &[(f32, f32, f32)] = &[
        (0.9, 0.7, 0.1),  // Yellow school bus
        (0.2, 0.4, 0.7),  // Blue city bus
        (0.8, 0.3, 0.1),  // Orange transit
        (0.1, 0.5, 0.3),  // Green eco bus
        (0.9, 0.9, 0.9),  // White shuttle
    ];

    let terrain = TerrainSampler::new(&terrain_config);

    // Spawn vehicles up to target count
    let to_spawn = (config.target_count - current_count).min(5); // Spawn max 5 per frame

    for _ in 0..to_spawn {
        // Pick random vehicle type
        let vehicle_type = VehicleType::random(rng);
        let (length, width, height) = vehicle_type.dimensions();

        // Pick random intersection
        let start_node_idx = intersections[rng.gen_range(0..intersections.len())];

        // Get edges from this node
        let edges: Vec<EdgeIndex> = road_graph.edges_of_node(start_node_idx).collect();
        if edges.is_empty() {
            continue;
        }

        // Pick random edge (buses prefer major roads)
        let edge_idx = if vehicle_type == VehicleType::Bus {
            // Prefer major roads for buses
            let major_edges: Vec<EdgeIndex> = edges.iter().copied().filter(|&e| {
                road_graph.edge_by_index(e).map(|edge| {
                    matches!(edge.road_type, RoadType::Major | RoadType::Highway)
                }).unwrap_or(false)
            }).collect();
            if !major_edges.is_empty() {
                major_edges[rng.gen_range(0..major_edges.len())]
            } else {
                edges[rng.gen_range(0..edges.len())]
            }
        } else {
            edges[rng.gen_range(0..edges.len())]
        };

        let Some((node_a, node_b)) = road_graph.edge_endpoints(edge_idx) else {
            continue;
        };

        // Determine direction (forward = heading away from start_node)
        let (forward, dest_node) = if node_a == start_node_idx {
            (true, node_b)
        } else {
            (false, node_a)
        };

        // Random speed with variation, adjusted by vehicle type
        let speed_mult = 1.0 + rng.gen_range(-config.speed_variation..config.speed_variation);
        let target_speed = config.base_speed * speed_mult * vehicle_type.speed_multiplier();

        // Get road type to adjust speed
        let edge = road_graph.edge_by_index(edge_idx).unwrap();
        let road_speed_mult = match edge.road_type {
            RoadType::Highway => 1.5,
            RoadType::Major => 1.0,
            RoadType::Minor => 0.8,
            RoadType::Alley => 0.5,
        };
        let final_speed = target_speed * road_speed_mult;

        // Pick color based on vehicle type
        let (r, g, b) = if vehicle_type == VehicleType::Bus {
            bus_colors[rng.gen_range(0..bus_colors.len())]
        } else {
            vehicle_type.body_color(rng)
        };

        // Car/bus paint is NOT metallic - it's clear coat over pigment
        let body_material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.5,  // Moderate shine, not mirror-like
            metallic: 0.0,              // Vehicle paint is dielectric, not metallic
            reflectance: 0.35,          // Standard clear coat reflectance
            ..default()
        });

        // Create realistic vehicle mesh using procedural generator
        let body_mesh = meshes.add(generate_vehicle_mesh(&vehicle_type.mesh_config()));

        // Get initial position
        let points = &edge.points;
        let (pos, dir) = if forward {
            interpolate_edge_position(points, 0.0)
        } else {
            let (p, d) = interpolate_edge_position(points, 1.0);
            (p, -d)
        };

        let terrain_height = terrain.sample(pos.x, pos.y);
        let road_surface = terrain_height + 0.12; // Road height offset
        let body_y = road_surface + height * 0.35;
        let angle = (-dir.x).atan2(-dir.y);
        let rotation = Quat::from_rotation_y(angle);

        // Spawn vehicle body with navigation and type
        let vehicle_entity = commands.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_material.clone()),
            Transform::from_xyz(pos.x, body_y, pos.y).with_rotation(rotation),
            MovingVehicle,
            vehicle_type,
            VehicleNavigation {
                current_edge: edge_idx,
                forward,
                progress: if forward { 0.0 } else { 1.0 },
                speed: final_speed,
                target_speed: final_speed,
                destination_node: dest_node,
                previous_node: Some(start_node_idx),
                stopping: false,
                // Lane offset: drive on the right side of the road
                // Offset depends on road type (wider roads = more offset)
                lane_offset: match edge.road_type {
                    RoadType::Highway => 3.0, // 3m from center
                    RoadType::Major => 2.0,   // 2m from center
                    RoadType::Minor => 1.5,   // 1.5m from center
                    RoadType::Alley => 0.0,   // Center for narrow alleys
                } * if forward { 1.0 } else { -1.0 }, // Right side based on direction
                target_lane_offset: match edge.road_type {
                    RoadType::Highway => 3.0,
                    RoadType::Major => 2.0,
                    RoadType::Minor => 1.5,
                    RoadType::Alley => 0.0,
                } * if forward { 1.0 } else { -1.0 },
            },
        )).id();

        // Add emergency siren for emergency vehicles
        if vehicle_type.is_emergency() {
            commands.entity(vehicle_entity).insert(EmergencySiren {
                phase: rng.gen::<f32>(), // Random starting phase
                frequency: 2.0,
                active: true,
            });

            // Spawn emergency lights on roof
            let light_bar_height = height + 0.2;
            let light_spread = width * 0.3;

            // Determine light colors based on vehicle type
            let (left_color, right_color) = match vehicle_type {
                VehicleType::PoliceCar => (EmergencyLightColor::Red, EmergencyLightColor::Blue),
                VehicleType::FireTruck => (EmergencyLightColor::Red, EmergencyLightColor::Red),
                VehicleType::Ambulance => (EmergencyLightColor::Red, EmergencyLightColor::Red),
                _ => (EmergencyLightColor::Red, EmergencyLightColor::Red),
            };

            // Left light
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.0, 0.0),
                    intensity: 50000.0,
                    range: 15.0,
                    radius: 0.1,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(pos.x - light_spread * dir.y, body_y + light_bar_height, pos.y + light_spread * dir.x),
                EmergencyLight {
                    vehicle: vehicle_entity,
                    position: 0,
                    color: left_color,
                },
            ));

            // Right light
            commands.spawn((
                PointLight {
                    color: Color::srgb(0.0, 0.0, 1.0),
                    intensity: 50000.0,
                    range: 15.0,
                    radius: 0.1,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(pos.x + light_spread * dir.y, body_y + light_bar_height, pos.y - light_spread * dir.x),
                EmergencyLight {
                    vehicle: vehicle_entity,
                    position: 1,
                    color: right_color,
                },
            ));
        }

        // Note: Cabin is now integrated into the procedural vehicle mesh
        // No separate cabin entity needed
    }

    if current_count + to_spawn >= config.target_count {
        initialized.0 = true;
        info!("Spawned {} moving vehicles (sedans, SUVs, trucks, vans, buses)", config.target_count);
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

        // Update lane offset for new road
        let new_lane_offset = if let Some(edge) = road_graph.edge_by_index(next_edge) {
            let base_offset = match edge.road_type {
                RoadType::Highway => 3.0,
                RoadType::Major => 2.0,
                RoadType::Minor => 1.5,
                RoadType::Alley => 0.0,
            };
            base_offset * if forward { 1.0 } else { -1.0 }
        } else {
            0.0
        };

        // Update navigation state
        nav.previous_node = Some(current_node);
        nav.current_edge = next_edge;
        nav.forward = forward;
        nav.progress = if forward { 0.0 } else { 1.0 };
        nav.destination_node = dest_node;
        nav.stopping = false;
        nav.target_lane_offset = new_lane_offset;
        // Smooth transition to new lane over time
    }
}

/// Smooth lane change system - interpolate current lane offset toward target.
fn vehicle_lane_change(
    time: Res<Time>,
    mut vehicles: Query<&mut VehicleNavigation, With<MovingVehicle>>,
) {
    let dt = time.delta_secs();
    let lane_change_speed = 2.0; // meters per second of lateral movement

    for mut nav in vehicles.iter_mut() {
        let diff = nav.target_lane_offset - nav.lane_offset;
        if diff.abs() > 0.01 {
            let change = diff.signum() * lane_change_speed * dt;
            if change.abs() > diff.abs() {
                nav.lane_offset = nav.target_lane_offset;
            } else {
                nav.lane_offset += change;
            }
        }
    }
}

/// Update vehicle transforms based on their navigation state.
fn vehicle_transform_sync(
    road_graph: Res<RoadGraph>,
    terrain_config: Res<TerrainConfig>,
    mut vehicles: Query<(&VehicleNavigation, &VehicleType, &mut Transform), With<MovingVehicle>>,
) {
    let terrain = TerrainSampler::new(&terrain_config);

    for (nav, vehicle_type, mut transform) in vehicles.iter_mut() {
        let Some(edge) = road_graph.edge_by_index(nav.current_edge) else {
            continue;
        };

        // Clamp progress to valid range
        let progress = nav.progress.clamp(0.0, 1.0);

        // Get position and direction along edge
        let (center_pos, mut dir) = interpolate_edge_position(&edge.points, progress);

        // Flip direction if traveling backward
        if !nav.forward {
            dir = -dir;
        }

        // Apply lane offset (perpendicular to road direction)
        // Positive offset = right side of road (in direction of travel)
        let perp = Vec2::new(-dir.y, dir.x); // Perpendicular vector
        let pos = center_pos + perp * nav.lane_offset;

        // Get vehicle height from type
        let (_, _, height) = vehicle_type.dimensions();

        // Update position with terrain and road height
        let terrain_height = terrain.sample(pos.x, pos.y);
        let road_surface = terrain_height + 0.12; // Road height offset
        let body_y = road_surface + height * 0.35;

        transform.translation.x = pos.x;
        transform.translation.y = body_y;
        transform.translation.z = pos.y;

        // Update rotation to face direction of travel
        if dir.length_squared() > 0.001 {
            let angle = (-dir.x).atan2(-dir.y);
            transform.rotation = Quat::from_rotation_y(angle);
        }
    }
}

/// Update emergency siren phase for flashing effect.
fn update_emergency_sirens(
    time: Res<Time>,
    mut sirens: Query<&mut EmergencySiren>,
) {
    let dt = time.delta_secs();

    for mut siren in sirens.iter_mut() {
        if siren.active {
            siren.phase = (siren.phase + dt * siren.frequency) % 1.0;
        }
    }
}

/// Update emergency light intensity and position based on siren phase.
fn update_emergency_lights(
    vehicle_query: Query<(&Transform, &VehicleType, &EmergencySiren), With<MovingVehicle>>,
    mut light_query: Query<(&EmergencyLight, &mut PointLight, &mut Transform), Without<MovingVehicle>>,
) {
    for (light, mut point_light, mut light_transform) in light_query.iter_mut() {
        // Get the parent vehicle's transform and siren state
        let Ok((vehicle_transform, vehicle_type, siren)) = vehicle_query.get(light.vehicle) else {
            continue;
        };

        // Calculate flashing intensity based on siren phase and light position
        // Left and right lights alternate
        let flash_phase = if light.position == 0 {
            siren.phase
        } else {
            (siren.phase + 0.5) % 1.0
        };

        // Sharp on/off flashing
        let intensity = if flash_phase < 0.5 {
            80000.0 // Bright
        } else {
            5000.0 // Dim but not off
        };

        point_light.intensity = intensity;

        // Update color based on light type
        point_light.color = match light.color {
            EmergencyLightColor::Red => Color::srgb(1.0, 0.1, 0.05),
            EmergencyLightColor::Blue => Color::srgb(0.1, 0.2, 1.0),
        };

        // Update position to follow vehicle
        let (_, width, height) = vehicle_type.dimensions();
        let right = vehicle_transform.right();
        let light_spread = width * 0.3;
        let light_bar_height = height * 0.5 + 0.3;

        let offset = if light.position == 0 {
            -right * light_spread
        } else {
            right * light_spread
        };

        light_transform.translation = vehicle_transform.translation
            + offset
            + Vec3::Y * light_bar_height;
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

//! Pedestrian movement system.
//!
//! Spawns pedestrians that walk along sidewalks, following the road network
//! with perpendicular offsets from road centerlines.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use petgraph::graph::{EdgeIndex, NodeIndex};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadNodeType, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct PedestrianPlugin;

impl Plugin for PedestrianPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PedestrianConfig>()
            .init_resource::<PedestriansInitialized>()
            .add_systems(
                Update,
                (
                    spawn_pedestrians.run_if(should_spawn_pedestrians),
                    pedestrian_movement,
                    pedestrian_edge_transition,
                    pedestrian_transform_sync,
                )
                    .chain(),
            );
    }
}

/// Marker component for pedestrians.
#[derive(Component)]
pub struct Pedestrian;

/// Navigation state for a moving pedestrian.
#[derive(Component)]
pub struct PedestrianNavigation {
    pub current_edge: EdgeIndex,
    pub forward: bool,
    pub progress: f32,
    pub speed: f32,
    pub side: f32,  // 1.0 or -1.0 for left/right sidewalk
    pub destination_node: NodeIndex,
    pub previous_node: Option<NodeIndex>,
}

/// Configuration for pedestrians.
#[derive(Resource)]
pub struct PedestrianConfig {
    pub target_count: usize,
    pub base_speed: f32,
    pub speed_variance: f32,
    pub body_height: f32,
    pub body_radius: f32,
    pub head_radius: f32,
    pub sidewalk_offset: f32,
    pub seed: u64,
}

impl Default for PedestrianConfig {
    fn default() -> Self {
        Self {
            target_count: 50,
            base_speed: 1.4,         // ~5 km/h walking speed
            speed_variance: 0.3,     // Some walk faster/slower
            body_height: 1.5,
            body_radius: 0.2,
            head_radius: 0.15,
            sidewalk_offset: 4.0,    // Distance from road center to sidewalk
            seed: 88888,
        }
    }
}

/// Marker that pedestrian spawning has been initialized.
#[derive(Resource, Default)]
pub struct PedestriansInitialized(pub bool);

/// Run condition: spawn pedestrians when roads exist and we haven't reached target count.
fn should_spawn_pedestrians(
    road_mesh_query: Query<&RoadMeshGenerated>,
    pedestrian_query: Query<&Pedestrian>,
    config: Res<PedestrianConfig>,
    initialized: Res<PedestriansInitialized>,
) -> bool {
    !road_mesh_query.is_empty()
        && (pedestrian_query.iter().count() < config.target_count || !initialized.0)
}

// Clothing color palette
const CLOTHING_COLORS: &[(f32, f32, f32)] = &[
    (0.2, 0.3, 0.5),   // Blue jacket
    (0.5, 0.2, 0.2),   // Red coat
    (0.3, 0.3, 0.3),   // Gray suit
    (0.15, 0.4, 0.2),  // Green sweater
    (0.6, 0.5, 0.3),   // Tan coat
    (0.1, 0.1, 0.15),  // Dark navy
    (0.4, 0.35, 0.3),  // Brown jacket
    (0.25, 0.25, 0.3), // Charcoal
];

// Skin tone palette
const SKIN_TONES: &[(f32, f32, f32)] = &[
    (0.96, 0.80, 0.69), // Light
    (0.87, 0.72, 0.53), // Light-medium
    (0.76, 0.57, 0.42), // Medium
    (0.55, 0.38, 0.28), // Medium-dark
    (0.36, 0.25, 0.18), // Dark
];

fn spawn_pedestrians(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<PedestrianConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pedestrian_query: Query<&Pedestrian>,
    mut initialized: ResMut<PedestriansInitialized>,
    mut local_rng: Local<Option<StdRng>>,
) {
    let rng = local_rng.get_or_insert_with(|| StdRng::seed_from_u64(config.seed));

    let current_count = pedestrian_query.iter().count();
    if current_count >= config.target_count {
        initialized.0 = true;
        return;
    }

    // Collect valid nodes (intersections or endpoints on Major/Minor roads)
    let valid_nodes: Vec<NodeIndex> = road_graph
        .nodes()
        .filter_map(|(idx, node)| {
            let neighbor_count = road_graph.neighbors(idx).count();
            if neighbor_count >= 1 && node.node_type != RoadNodeType::DeadEnd {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    if valid_nodes.is_empty() {
        warn!("No valid nodes found for pedestrian spawning");
        initialized.0 = true;
        return;
    }

    // Create meshes
    let body_mesh = meshes.add(Cylinder::new(config.body_radius, config.body_height));
    let head_mesh = meshes.add(Sphere::new(config.head_radius));

    let terrain = TerrainSampler::new(&terrain_config);

    // Spawn pedestrians up to target count
    let to_spawn = (config.target_count - current_count).min(10);

    for _ in 0..to_spawn {
        // Pick random starting node
        let start_node_idx = valid_nodes[rng.gen_range(0..valid_nodes.len())];

        // Get edges from this node (prefer Major/Minor roads with sidewalks)
        let edges: Vec<EdgeIndex> = road_graph
            .edges_of_node(start_node_idx)
            .filter(|&e| {
                if let Some(edge) = road_graph.edge_by_index(e) {
                    matches!(edge.road_type, RoadType::Major | RoadType::Minor)
                } else {
                    false
                }
            })
            .collect();

        if edges.is_empty() {
            continue;
        }

        // Pick random edge
        let edge_idx = edges[rng.gen_range(0..edges.len())];
        let Some((node_a, node_b)) = road_graph.edge_endpoints(edge_idx) else {
            continue;
        };

        // Determine direction
        let (forward, dest_node) = if node_a == start_node_idx {
            (true, node_b)
        } else {
            (false, node_a)
        };

        // Random speed with variance
        let speed = config.base_speed + rng.gen_range(-config.speed_variance..config.speed_variance);

        // Random side of street
        let side = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

        // Random clothing color
        let (cr, cg, cb) = CLOTHING_COLORS[rng.gen_range(0..CLOTHING_COLORS.len())];
        let body_material = materials.add(StandardMaterial {
            base_color: Color::srgb(cr, cg, cb),
            perceptual_roughness: 0.8,
            ..default()
        });

        // Random skin tone
        let (sr, sg, sb) = SKIN_TONES[rng.gen_range(0..SKIN_TONES.len())];
        let head_material = materials.add(StandardMaterial {
            base_color: Color::srgb(sr, sg, sb),
            perceptual_roughness: 0.9,
            ..default()
        });

        // Get initial position
        let edge = road_graph.edge_by_index(edge_idx).unwrap();
        let points = &edge.points;
        let initial_progress = if forward { 0.0 } else { 1.0 };
        let (pos, dir) = interpolate_edge_position(points, initial_progress);

        // Calculate sidewalk position
        let perp = Vec2::new(-dir.y, dir.x);
        let sidewalk_pos = pos + perp * config.sidewalk_offset * side;

        let terrain_height = terrain.sample(sidewalk_pos.x, sidewalk_pos.y);
        let body_y = terrain_height + config.body_height / 2.0;
        let head_y = terrain_height + config.body_height + config.head_radius;

        // Calculate facing direction
        let facing_dir = if forward { dir } else { -dir };
        let angle = facing_dir.y.atan2(facing_dir.x);
        let rotation = Quat::from_rotation_y(-angle);

        // Spawn body with navigation
        commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(body_material),
            Transform::from_xyz(sidewalk_pos.x, body_y, sidewalk_pos.y).with_rotation(rotation),
            Pedestrian,
            PedestrianNavigation {
                current_edge: edge_idx,
                forward,
                progress: initial_progress,
                speed,
                side,
                destination_node: dest_node,
                previous_node: Some(start_node_idx),
            },
        ));

        // Spawn head (follows the body)
        commands.spawn((
            Mesh3d(head_mesh.clone()),
            MeshMaterial3d(head_material),
            Transform::from_xyz(sidewalk_pos.x, head_y, sidewalk_pos.y),
            Pedestrian,
        ));
    }

    if current_count + to_spawn >= config.target_count {
        initialized.0 = true;
        info!("Spawned {} pedestrians", config.target_count);
    }
}

/// Advance pedestrian progress along their current edge.
fn pedestrian_movement(
    time: Res<Time>,
    road_graph: Res<RoadGraph>,
    mut pedestrians: Query<&mut PedestrianNavigation, With<Pedestrian>>,
) {
    let dt = time.delta_secs();

    for mut nav in pedestrians.iter_mut() {
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

/// Handle pedestrians reaching the end of their current edge.
fn pedestrian_edge_transition(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    mut pedestrians: Query<(Entity, &mut PedestrianNavigation), With<Pedestrian>>,
    mut local_rng: Local<Option<StdRng>>,
) {
    let rng = local_rng.get_or_insert_with(|| StdRng::seed_from_u64(66666));

    for (entity, mut nav) in pedestrians.iter_mut() {
        // Check if we've reached the end of the edge
        let at_end = (nav.forward && nav.progress >= 1.0) || (!nav.forward && nav.progress <= 0.0);

        if !at_end {
            continue;
        }

        let current_node = nav.destination_node;

        // Get all edges from this node (prefer Major/Minor roads)
        let edges: Vec<EdgeIndex> = road_graph
            .edges_of_node(current_node)
            .filter(|&e| {
                if e == nav.current_edge {
                    return false; // Avoid U-turns
                }
                if let Some(edge) = road_graph.edge_by_index(e) {
                    matches!(edge.road_type, RoadType::Major | RoadType::Minor)
                } else {
                    false
                }
            })
            .collect();

        if edges.is_empty() {
            // Dead end or no suitable roads - despawn and respawn elsewhere
            commands.entity(entity).despawn();
            continue;
        }

        // Pick random next edge
        let next_edge = edges[rng.gen_range(0..edges.len())];
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

        // Update navigation state
        nav.previous_node = Some(current_node);
        nav.current_edge = next_edge;
        nav.forward = forward;
        nav.progress = if forward { 0.0 } else { 1.0 };
        nav.destination_node = dest_node;
    }
}

/// Update pedestrian transforms based on their navigation state.
fn pedestrian_transform_sync(
    road_graph: Res<RoadGraph>,
    terrain_config: Res<TerrainConfig>,
    config: Res<PedestrianConfig>,
    mut pedestrians: Query<(&PedestrianNavigation, &mut Transform), With<Pedestrian>>,
) {
    let terrain = TerrainSampler::new(&terrain_config);

    for (nav, mut transform) in pedestrians.iter_mut() {
        let Some(edge) = road_graph.edge_by_index(nav.current_edge) else {
            continue;
        };

        // Clamp progress to valid range
        let progress = nav.progress.clamp(0.0, 1.0);

        // Get position and direction along edge
        let (pos, dir) = interpolate_edge_position(&edge.points, progress);

        // Calculate sidewalk offset
        let perp = Vec2::new(-dir.y, dir.x);
        let sidewalk_pos = pos + perp * config.sidewalk_offset * nav.side;

        // Update position with terrain height
        let terrain_height = terrain.sample(sidewalk_pos.x, sidewalk_pos.y);
        let body_y = terrain_height + config.body_height / 2.0;

        transform.translation.x = sidewalk_pos.x;
        transform.translation.y = body_y;
        transform.translation.z = sidewalk_pos.y;

        // Update rotation to face direction of travel
        let facing_dir = if nav.forward { dir } else { -dir };
        if facing_dir.length_squared() > 0.001 {
            let angle = facing_dir.y.atan2(facing_dir.x);
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

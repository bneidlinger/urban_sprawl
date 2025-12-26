//! Road network generator using tensor fields and streamline integration.
//!
//! Creates organic city road layouts by:
//! 1. Composing a tensor field (radial downtown + grid suburbs)
//! 2. Tracing streamlines through the field
//! 3. Building a graph with snapped intersections

use bevy::prelude::*;
use smallvec::SmallVec;

use super::roads::{RoadEdge, RoadGraph, RoadNodeType, RoadType};
use super::streamline::{generate_seeds, Streamline, StreamlineConfig, StreamlineIntegrator};
use super::tensor::{BasisField, TensorField};

/// Configuration for road generation.
#[derive(Resource, Clone)]
pub struct RoadGenConfig {
    /// Size of the city in world units.
    pub city_size: f32,
    /// Center of downtown (radial field origin).
    pub downtown_center: Vec2,
    /// Strength of the radial field decay.
    pub radial_decay: f32,
    /// Base grid angle (radians).
    pub grid_angle: f32,
    /// Streamline configuration.
    pub streamline: StreamlineConfig,
    /// Minimum road segment length.
    pub min_segment_length: f32,
}

impl Default for RoadGenConfig {
    fn default() -> Self {
        Self {
            city_size: 500.0,
            downtown_center: Vec2::ZERO,
            radial_decay: 0.008,
            grid_angle: 0.0,
            streamline: StreamlineConfig {
                step_size: 2.0,
                max_steps: 300,
                separation: 15.0,
                snap_distance: 8.0,
            },
            min_segment_length: 5.0,
        }
    }
}

/// Event to trigger road generation.
#[derive(Event)]
pub struct GenerateRoadsEvent;

/// Marker that roads have been generated.
#[derive(Resource, Default)]
pub struct RoadsGenerated(pub bool);

pub struct RoadGeneratorPlugin;

impl Plugin for RoadGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadGenConfig>()
            .init_resource::<RoadsGenerated>()
            .add_event::<GenerateRoadsEvent>()
            .add_systems(Update, generate_roads_on_event)
            .add_systems(Startup, trigger_initial_generation);
    }
}

fn trigger_initial_generation(mut events: EventWriter<GenerateRoadsEvent>) {
    events.send(GenerateRoadsEvent);
}

fn generate_roads_on_event(
    mut events: EventReader<GenerateRoadsEvent>,
    mut tensor_field: ResMut<TensorField>,
    mut road_graph: ResMut<RoadGraph>,
    config: Res<RoadGenConfig>,
    mut generated: ResMut<RoadsGenerated>,
) {
    for _ in events.read() {
        info!("Generating road network...");

        // Clear existing
        *road_graph = RoadGraph::default();
        tensor_field.basis_fields.clear();

        // Build the tensor field
        build_tensor_field(&mut tensor_field, &config);

        // Generate the road network
        generate_road_network(&tensor_field, &mut road_graph, &config);

        generated.0 = true;

        info!(
            "Road generation complete: {} nodes, {} edges",
            road_graph.node_count(),
            road_graph.edge_count()
        );
    }
}

/// Build a tensor field with downtown radial + grid suburbs.
fn build_tensor_field(field: &mut TensorField, config: &RoadGenConfig) {
    // Global grid (aligned to axes or slight angle)
    field.add_grid(config.grid_angle);

    // Downtown radial field
    field.add_radial(config.downtown_center, config.radial_decay);

    // Could add more features:
    // - Polylines for rivers, highways
    // - Secondary radial centers for districts
}

/// Generate road network by tracing streamlines.
fn generate_road_network(field: &TensorField, graph: &mut RoadGraph, config: &RoadGenConfig) {
    let half_size = config.city_size / 2.0;
    let bounds = Rect::new(-half_size, -half_size, half_size, half_size);

    // Generate seed points
    let seed_spacing = config.streamline.separation * 2.0;
    let seeds = generate_seeds(bounds, seed_spacing);

    let integrator = StreamlineIntegrator::new(field, config.streamline.clone());

    // Track existing streamlines for separation checking
    let mut all_streamlines: Vec<Streamline> = Vec::new();

    // Trace major roads (following major eigenvector)
    for seed in &seeds {
        if !is_valid_seed(*seed, &all_streamlines, config.streamline.separation) {
            continue;
        }

        let streamline = integrator.trace(*seed, true);
        if streamline.points.len() >= 3 {
            add_streamline_to_graph(&streamline, graph, config, RoadType::Major);
            all_streamlines.push(streamline);
        }
    }

    // Trace minor roads (following minor eigenvector - cross streets)
    for seed in &seeds {
        if !is_valid_seed(*seed, &all_streamlines, config.streamline.separation * 0.7) {
            continue;
        }

        let streamline = integrator.trace(*seed, false);
        if streamline.points.len() >= 3 {
            add_streamline_to_graph(&streamline, graph, config, RoadType::Minor);
            all_streamlines.push(streamline);
        }
    }
}

/// Check if a seed point is far enough from existing streamlines.
fn is_valid_seed(seed: Vec2, streamlines: &[Streamline], min_distance: f32) -> bool {
    for streamline in streamlines {
        for point in &streamline.points {
            if seed.distance(point.position) < min_distance {
                return false;
            }
        }
    }
    true
}

/// Convert a streamline to road graph edges.
fn add_streamline_to_graph(
    streamline: &Streamline,
    graph: &mut RoadGraph,
    config: &RoadGenConfig,
    road_type: RoadType,
) {
    if streamline.points.len() < 2 {
        return;
    }

    let snap_dist = config.streamline.snap_distance;

    // Start node
    let start_pos = streamline.points[0].position;
    let mut prev_node = graph.snap_or_create(start_pos, snap_dist, RoadNodeType::Endpoint);
    let mut segment_points: SmallVec<[Vec2; 8]> = SmallVec::new();
    segment_points.push(start_pos);

    // Process each point
    for point in streamline.points.iter().skip(1) {
        let pos = point.position;
        segment_points.push(pos);

        // Check if we should create an intersection (near existing node)
        if let Some(existing) = graph.find_nearest(pos, snap_dist) {
            if existing != prev_node {
                // Create edge to existing node
                graph.add_edge(prev_node, existing, segment_points.clone(), road_type);
                prev_node = existing;
                segment_points.clear();
                segment_points.push(pos);
            }
        }

        // Check segment length - create intermediate nodes for long segments
        let segment_length: f32 = segment_points
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum();

        if segment_length > config.streamline.separation * 3.0 {
            // Create intermediate node
            let new_node = graph.add_node(pos, RoadNodeType::Intersection);
            graph.add_edge(prev_node, new_node, segment_points.clone(), road_type);
            prev_node = new_node;
            segment_points.clear();
            segment_points.push(pos);
        }
    }

    // Final node
    let end_pos = streamline.points.last().unwrap().position;
    let end_node = graph.snap_or_create(end_pos, snap_dist, RoadNodeType::Endpoint);
    if end_node != prev_node && segment_points.len() >= 2 {
        graph.add_edge(prev_node, end_node, segment_points, road_type);
    }
}

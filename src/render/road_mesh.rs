//! Road mesh generation from road graph.
//!
//! Converts road edges into renderable quad strips with proper width.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};
use petgraph::graph::NodeIndex;

use crate::procgen::road_generator::RoadsGenerated;
use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::instancing::TerrainConfig;

pub struct RoadMeshPlugin;

impl Plugin for RoadMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadMeshConfig>()
            .add_systems(Update, generate_road_meshes.run_if(should_generate_meshes));
    }
}

fn should_generate_meshes(
    generated: Res<RoadsGenerated>,
    query: Query<&RoadMeshGenerated>,
) -> bool {
    generated.0 && query.is_empty()
}

/// Marker that road meshes have been generated.
#[derive(Component)]
pub struct RoadMeshGenerated;

/// Marker for road mesh entities.
#[derive(Component)]
pub struct RoadMesh {
    pub road_type: RoadType,
}

/// Marker for intersection mesh entities.
#[derive(Component)]
pub struct IntersectionMesh;

/// Marker for sidewalk mesh entities.
#[derive(Component)]
pub struct SidewalkMesh;

/// Configuration for road mesh generation.
#[derive(Resource)]
pub struct RoadMeshConfig {
    pub highway_width: f32,
    pub major_width: f32,
    pub minor_width: f32,
    pub alley_width: f32,
    pub road_height: f32,
    pub sidewalk_width: f32,
}

impl Default for RoadMeshConfig {
    fn default() -> Self {
        Self {
            highway_width: 12.0,
            major_width: 8.0,
            minor_width: 5.0,
            alley_width: 3.0,
            road_height: 0.1,
            sidewalk_width: 2.0,
        }
    }
}

/// Generate road meshes from the road graph.
fn generate_road_meshes(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<RoadMeshConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Generating road meshes...");

    // Create terrain sampler with same seed as terrain
    let terrain = TerrainSampler::new(&terrain_config);

    // Road surface material
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.4),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Sidewalk material (lighter concrete)
    let sidewalk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.63, 0.60),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Generate mesh for each edge
    for edge in road_graph.edges() {
        if edge.points.len() < 2 {
            continue;
        }

        let width = match edge.road_type {
            RoadType::Highway => config.highway_width,
            RoadType::Major => config.major_width,
            RoadType::Minor => config.minor_width,
            RoadType::Alley => config.alley_width,
        };

        let mesh = create_road_strip_mesh(&edge.points, width, config.road_height, &terrain);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(road_material.clone()),
            Transform::IDENTITY,
            RoadMesh {
                road_type: edge.road_type,
            },
        ));

        // Add sidewalks (not for alleys or highways)
        if edge.road_type == RoadType::Major || edge.road_type == RoadType::Minor {
            let sidewalk_offset = width / 2.0 + config.sidewalk_width / 2.0;

            // Left sidewalk
            let left_points = offset_polyline(&edge.points, sidewalk_offset);
            let left_mesh = create_road_strip_mesh(&left_points, config.sidewalk_width, config.road_height + 0.05, &terrain);
            commands.spawn((
                Mesh3d(meshes.add(left_mesh)),
                MeshMaterial3d(sidewalk_material.clone()),
                Transform::IDENTITY,
                SidewalkMesh,
            ));

            // Right sidewalk
            let right_points = offset_polyline(&edge.points, -sidewalk_offset);
            let right_mesh = create_road_strip_mesh(&right_points, config.sidewalk_width, config.road_height + 0.05, &terrain);
            commands.spawn((
                Mesh3d(meshes.add(right_mesh)),
                MeshMaterial3d(sidewalk_material.clone()),
                Transform::IDENTITY,
                SidewalkMesh,
            ));
        }
    }

    // Generate proper intersection meshes
    for (node_idx, node) in road_graph.nodes() {
        let neighbors: Vec<NodeIndex> = road_graph.graph.neighbors(node_idx).collect();

        if neighbors.len() < 2 {
            continue; // Not a real intersection
        }

        // Collect directions to all connected roads
        let mut road_directions: Vec<(Vec2, f32)> = Vec::new();

        for neighbor_idx in &neighbors {
            if let Some(neighbor_node) = road_graph.graph.node_weight(*neighbor_idx) {
                let dir = (neighbor_node.position - node.position).normalize_or_zero();
                // Get the width of this road connection
                let width = get_road_width_at_node(&road_graph, node_idx, *neighbor_idx, &config);
                road_directions.push((dir, width));
            }
        }

        if road_directions.len() >= 2 {
            let mesh = create_intersection_mesh(
                node.position,
                &road_directions,
                config.road_height,
                &terrain,
            );

            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(road_material.clone()),
                Transform::IDENTITY,
                IntersectionMesh,
            ));
        }
    }

    // Marker entity to prevent re-generation
    commands.spawn(RoadMeshGenerated);

    info!("Road meshes generated");
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

    /// Sample terrain height at world position (x, z).
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

/// Get the road width for an edge connecting two nodes.
fn get_road_width_at_node(
    graph: &RoadGraph,
    from: NodeIndex,
    to: NodeIndex,
    config: &RoadMeshConfig,
) -> f32 {
    // Find the edge between these nodes
    if let Some(edge_idx) = graph.graph.find_edge(from, to) {
        if let Some(edge) = graph.graph.edge_weight(edge_idx) {
            return match edge.road_type {
                RoadType::Highway => config.highway_width,
                RoadType::Major => config.major_width,
                RoadType::Minor => config.minor_width,
                RoadType::Alley => config.alley_width,
            };
        }
    }
    config.minor_width // Default
}

/// Create intersection mesh as a convex polygon connecting all road endpoints.
fn create_intersection_mesh(
    center: Vec2,
    road_directions: &[(Vec2, f32)],
    height_offset: f32,
    terrain: &TerrainSampler,
) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Create corner points for each road connection
    let mut corner_points: Vec<Vec2> = Vec::new();

    for (dir, width) in road_directions {
        let half_width = width / 2.0;
        let perp = Vec2::new(-dir.y, dir.x);

        // Two corners where this road meets the intersection
        let left = center + *dir * half_width * 0.5 + perp * half_width;
        let right = center + *dir * half_width * 0.5 - perp * half_width;

        corner_points.push(left);
        corner_points.push(right);
    }

    // Sort points by angle around center for proper polygon winding
    corner_points.sort_by(|a, b| {
        let angle_a = (a.y - center.y).atan2(a.x - center.x);
        let angle_b = (b.y - center.y).atan2(b.x - center.x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    // Add center vertex with terrain height
    let center_height = terrain.sample(center.x, center.y) + height_offset + 0.02;
    vertices.push([center.x, center_height, center.y]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Add corner vertices with terrain height
    for point in &corner_points {
        let point_height = terrain.sample(point.x, point.y) + height_offset + 0.02;
        vertices.push([point.x, point_height, point.y]);
        normals.push([0.0, 1.0, 0.0]);

        let uv_x = (point.x - center.x) / 20.0 + 0.5;
        let uv_y = (point.y - center.y) / 20.0 + 0.5;
        uvs.push([uv_x, uv_y]);
    }

    // Create triangles (fan from center)
    let num_corners = corner_points.len() as u32;
    for i in 0..num_corners {
        let next = (i + 1) % num_corners;
        // CCW winding: center, current, next
        indices.push(0);
        indices.push(i + 1);
        indices.push(next + 1);
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a quad strip mesh for a road segment.
fn create_road_strip_mesh(points: &[Vec2], width: f32, height_offset: f32, terrain: &TerrainSampler) -> Mesh {
    let half_width = width / 2.0;
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut accumulated_length = 0.0;

    for i in 0..points.len() {
        let current = points[i];

        // Calculate direction (tangent)
        let tangent = if i == 0 {
            (points[1] - points[0]).normalize_or_zero()
        } else if i == points.len() - 1 {
            (points[i] - points[i - 1]).normalize_or_zero()
        } else {
            let incoming = (points[i] - points[i - 1]).normalize_or_zero();
            let outgoing = (points[i + 1] - points[i]).normalize_or_zero();
            ((incoming + outgoing) / 2.0).normalize_or_zero()
        };

        // Perpendicular
        let perp = Vec2::new(-tangent.y, tangent.x);

        // Left and right vertices
        let left = current + perp * half_width;
        let right = current - perp * half_width;

        // Sample terrain height at each vertex
        let left_height = terrain.sample(left.x, left.y) + height_offset;
        let right_height = terrain.sample(right.x, right.y) + height_offset;

        vertices.push([left.x, left_height, left.y]);
        vertices.push([right.x, right_height, right.y]);

        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);

        if i > 0 {
            accumulated_length += points[i].distance(points[i - 1]);
        }
        let u = accumulated_length / width;
        uvs.push([u, 0.0]);
        uvs.push([u, 1.0]);

        // Create triangles (CCW winding)
        if i > 0 {
            let base = (i as u32 - 1) * 2;
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 3);
            indices.push(base + 1);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Offset a polyline perpendicular to its direction.
fn offset_polyline(points: &[Vec2], offset: f32) -> Vec<Vec2> {
    points
        .iter()
        .enumerate()
        .map(|(i, &point)| {
            let tangent = if i == 0 {
                (points[1] - points[0]).normalize_or_zero()
            } else if i == points.len() - 1 {
                (points[i] - points[i - 1]).normalize_or_zero()
            } else {
                let incoming = (points[i] - points[i - 1]).normalize_or_zero();
                let outgoing = (points[i + 1] - points[i]).normalize_or_zero();
                ((incoming + outgoing) / 2.0).normalize_or_zero()
            };

            let perp = Vec2::new(-tangent.y, tangent.x);
            point + perp * offset
        })
        .collect()
}

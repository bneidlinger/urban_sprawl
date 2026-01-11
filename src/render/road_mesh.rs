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
use crate::tools::road_draw::RoadMeshDirty;

pub struct RoadMeshPlugin;

impl Plugin for RoadMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadMeshConfig>()
            .add_systems(Update, generate_road_meshes.run_if(should_generate_meshes))
            .add_systems(Update, handle_road_mesh_dirty);
    }
}

/// Handle dynamic road mesh updates when roads are added/removed.
fn handle_road_mesh_dirty(
    mut commands: Commands,
    mut dirty_events: EventReader<RoadMeshDirty>,
    road_meshes: Query<Entity, With<RoadMesh>>,
    sidewalk_meshes: Query<Entity, With<SidewalkMesh>>,
    curb_meshes: Query<Entity, With<CurbMesh>>,
    intersection_meshes: Query<Entity, With<IntersectionMesh>>,
    marker: Query<Entity, With<RoadMeshGenerated>>,
) {
    if dirty_events.read().next().is_none() {
        return;
    }

    // Clear all events
    dirty_events.clear();

    info!("Regenerating road meshes...");

    // Despawn existing road meshes (including curbs)
    for entity in road_meshes.iter()
        .chain(sidewalk_meshes.iter())
        .chain(curb_meshes.iter())
        .chain(intersection_meshes.iter())
    {
        commands.entity(entity).despawn();
    }

    // Remove the generated marker so meshes will regenerate
    for entity in &marker {
        commands.entity(entity).despawn();
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

/// Marker for curb mesh entities.
#[derive(Component)]
pub struct CurbMesh;

/// Configuration for road mesh generation.
#[derive(Resource)]
pub struct RoadMeshConfig {
    pub highway_width: f32,
    pub major_width: f32,
    pub minor_width: f32,
    pub alley_width: f32,
    pub road_height: f32,
    pub sidewalk_width: f32,
    pub curb_height: f32,
    pub curb_width: f32,
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
            curb_height: 0.15,
            curb_width: 0.2,
        }
    }
}

/// Road material presets for different road types.
struct RoadMaterials {
    highway: Handle<StandardMaterial>,
    major: Handle<StandardMaterial>,
    minor: Handle<StandardMaterial>,
    alley: Handle<StandardMaterial>,
    sidewalk: Handle<StandardMaterial>,
    curb: Handle<StandardMaterial>,
}

impl RoadMaterials {
    fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        // Highway - dark, fresh asphalt
        let highway = materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.18, 0.20),
            perceptual_roughness: 0.85,
            metallic: 0.0,
            reflectance: 0.3,
            ..default()
        });

        // Major roads - standard asphalt, slightly worn
        let major = materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.28, 0.30),
            perceptual_roughness: 0.9,
            metallic: 0.0,
            reflectance: 0.25,
            ..default()
        });

        // Minor roads - lighter, more worn asphalt
        let minor = materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.34, 0.36),
            perceptual_roughness: 0.92,
            metallic: 0.0,
            reflectance: 0.2,
            ..default()
        });

        // Alleys - very worn, almost concrete-like
        let alley = materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.40, 0.38),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            reflectance: 0.15,
            ..default()
        });

        // Sidewalk - concrete with subtle warmth
        let sidewalk = materials.add(StandardMaterial {
            base_color: Color::srgb(0.68, 0.65, 0.60),
            perceptual_roughness: 0.88,
            metallic: 0.0,
            reflectance: 0.2,
            ..default()
        });

        // Curb - slightly darker concrete, more weathered
        let curb = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.52, 0.48),
            perceptual_roughness: 0.92,
            metallic: 0.0,
            reflectance: 0.15,
            ..default()
        });

        Self {
            highway,
            major,
            minor,
            alley,
            sidewalk,
            curb,
        }
    }

    fn get_road_material(&self, road_type: RoadType) -> Handle<StandardMaterial> {
        match road_type {
            RoadType::Highway => self.highway.clone(),
            RoadType::Major => self.major.clone(),
            RoadType::Minor => self.minor.clone(),
            RoadType::Alley => self.alley.clone(),
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

    // Create road materials with distinct appearance per road type
    let road_mats = RoadMaterials::new(&mut materials);

    // Generate mesh for each edge
    for edge in road_graph.edges() {
        if edge.points.len() < 2 {
            continue;
        }

        // Skip edges that cross water - bridges handle those
        if edge.crosses_water {
            continue;
        }

        let width = match edge.road_type {
            RoadType::Highway => config.highway_width,
            RoadType::Major => config.major_width,
            RoadType::Minor => config.minor_width,
            RoadType::Alley => config.alley_width,
        };

        let mesh = create_road_strip_mesh(&edge.points, width, config.road_height, &terrain);

        // Use road type-specific material
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(road_mats.get_road_material(edge.road_type)),
            Transform::IDENTITY,
            RoadMesh {
                road_type: edge.road_type,
            },
        ));

        // Add sidewalks and curbs (not for alleys or highways)
        if edge.road_type == RoadType::Major || edge.road_type == RoadType::Minor {
            let curb_offset = width / 2.0 + config.curb_width / 2.0;
            let sidewalk_offset = width / 2.0 + config.curb_width + config.sidewalk_width / 2.0;
            let sidewalk_height = config.road_height + config.curb_height;

            // Left curb (between road and left sidewalk)
            let left_curb_points = offset_polyline(&edge.points, curb_offset);
            let left_curb_mesh = create_curb_strip_mesh(
                &left_curb_points,
                config.curb_width,
                config.curb_height,
                config.road_height,
                1.0, // Left side
                &terrain,
            );
            commands.spawn((
                Mesh3d(meshes.add(left_curb_mesh)),
                MeshMaterial3d(road_mats.curb.clone()),
                Transform::IDENTITY,
                CurbMesh,
            ));

            // Left sidewalk (raised to curb height)
            let left_points = offset_polyline(&edge.points, sidewalk_offset);
            let left_mesh = create_road_strip_mesh(&left_points, config.sidewalk_width, sidewalk_height, &terrain);
            commands.spawn((
                Mesh3d(meshes.add(left_mesh)),
                MeshMaterial3d(road_mats.sidewalk.clone()),
                Transform::IDENTITY,
                SidewalkMesh,
            ));

            // Right curb (between road and right sidewalk)
            let right_curb_points = offset_polyline(&edge.points, -curb_offset);
            let right_curb_mesh = create_curb_strip_mesh(
                &right_curb_points,
                config.curb_width,
                config.curb_height,
                config.road_height,
                -1.0, // Right side
                &terrain,
            );
            commands.spawn((
                Mesh3d(meshes.add(right_curb_mesh)),
                MeshMaterial3d(road_mats.curb.clone()),
                Transform::IDENTITY,
                CurbMesh,
            ));

            // Right sidewalk (raised to curb height)
            let right_points = offset_polyline(&edge.points, -sidewalk_offset);
            let right_mesh = create_road_strip_mesh(&right_points, config.sidewalk_width, sidewalk_height, &terrain);
            commands.spawn((
                Mesh3d(meshes.add(right_mesh)),
                MeshMaterial3d(road_mats.sidewalk.clone()),
                Transform::IDENTITY,
                SidewalkMesh,
            ));
        }
    }

    // Find the dominant road type at each intersection for material selection
    let get_intersection_road_type = |node_idx: NodeIndex| -> RoadType {
        let mut best_type = RoadType::Minor;
        for neighbor_idx in road_graph.graph.neighbors(node_idx) {
            if let Some(edge_idx) = road_graph.graph.find_edge(node_idx, neighbor_idx) {
                if let Some(edge) = road_graph.graph.edge_weight(edge_idx) {
                    // Prefer highway > major > minor > alley
                    best_type = match (best_type, edge.road_type) {
                        (_, RoadType::Highway) => RoadType::Highway,
                        (RoadType::Highway, _) => RoadType::Highway,
                        (_, RoadType::Major) => RoadType::Major,
                        (RoadType::Major, _) => RoadType::Major,
                        (_, RoadType::Minor) => RoadType::Minor,
                        (RoadType::Minor, _) => RoadType::Minor,
                        _ => RoadType::Alley,
                    };
                }
            }
        }
        best_type
    };

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

            // Use the dominant road type's material for this intersection
            let intersection_type = get_intersection_road_type(node_idx);
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(road_mats.get_road_material(intersection_type)),
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

/// Create a curb mesh along a polyline.
/// Curbs are 3D shapes with a vertical face (road-side), a top face, and inner face (sidewalk-side).
/// The `side` parameter: 1.0 for left curb (facing right), -1.0 for right curb (facing left).
fn create_curb_strip_mesh(
    points: &[Vec2],
    curb_width: f32,
    curb_height: f32,
    base_height: f32,
    side: f32,
    terrain: &TerrainSampler,
) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut accumulated_length = 0.0;

    for i in 0..points.len() {
        let current = points[i];

        // Calculate tangent direction
        let tangent = if i == 0 {
            (points[1] - points[0]).normalize_or_zero()
        } else if i == points.len() - 1 {
            (points[i] - points[i - 1]).normalize_or_zero()
        } else {
            let incoming = (points[i] - points[i - 1]).normalize_or_zero();
            let outgoing = (points[i + 1] - points[i]).normalize_or_zero();
            ((incoming + outgoing) / 2.0).normalize_or_zero()
        };

        // Perpendicular (pointing outward from road center based on side)
        let perp = Vec2::new(-tangent.y, tangent.x) * side;

        // Four vertices per point: bottom-inner, bottom-outer, top-inner, top-outer
        let inner = current;
        let outer = current + perp * curb_width;

        let terrain_height = terrain.sample(current.x, current.y);
        let bottom_height = terrain_height + base_height;
        let top_height = bottom_height + curb_height;

        // Vertices: 0=bottom-inner, 1=bottom-outer, 2=top-inner, 3=top-outer
        vertices.push([inner.x, bottom_height, inner.y]);
        vertices.push([outer.x, bottom_height, outer.y]);
        vertices.push([inner.x, top_height, inner.y]);
        vertices.push([outer.x, top_height, outer.y]);

        // Normals - horizontal facing outward for vertical faces
        let horizontal_normal = [perp.x, 0.0, perp.y];
        let up_normal = [0.0, 1.0, 0.0];
        let inward_normal = [-perp.x, 0.0, -perp.y];

        // For simplicity, assign normals based on face usage:
        // We'll use averaged normals for corners
        normals.push(inward_normal);  // bottom-inner: faces inward (toward road)
        normals.push(horizontal_normal); // bottom-outer
        normals.push(up_normal);      // top-inner: faces up
        normals.push(up_normal);      // top-outer: faces up

        if i > 0 {
            accumulated_length += points[i].distance(points[i - 1]);
        }
        let u = accumulated_length / 2.0; // UV tiling along length

        uvs.push([u, 0.0]);
        uvs.push([u, 0.0]);
        uvs.push([u, 1.0]);
        uvs.push([u, 1.0]);

        // Create triangles connecting this segment to the previous
        if i > 0 {
            let base = ((i - 1) * 4) as u32;
            let curr = (i * 4) as u32;

            // Outer vertical face (road-facing) - CCW winding
            // Uses vertices 1 (bottom-outer) and 3 (top-outer)
            indices.push(base + 1);
            indices.push(base + 3);
            indices.push(curr + 1);
            indices.push(curr + 1);
            indices.push(base + 3);
            indices.push(curr + 3);

            // Top face - CCW winding
            // Uses vertices 2 (top-inner) and 3 (top-outer)
            indices.push(base + 2);
            indices.push(curr + 2);
            indices.push(base + 3);
            indices.push(base + 3);
            indices.push(curr + 2);
            indices.push(curr + 3);

            // Inner vertical face (sidewalk-facing) - CCW winding
            // Uses vertices 0 (bottom-inner) and 2 (top-inner)
            indices.push(base + 0);
            indices.push(curr + 0);
            indices.push(base + 2);
            indices.push(base + 2);
            indices.push(curr + 0);
            indices.push(curr + 2);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

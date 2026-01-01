//! Road lane markings - center lines, edge lines, crosswalks.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct RoadMarkingsPlugin;

impl Plugin for RoadMarkingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarkingsConfig>()
            .add_systems(Update, generate_road_markings.run_if(should_generate_markings));
    }
}

fn should_generate_markings(
    road_mesh_query: Query<&RoadMeshGenerated>,
    markings_query: Query<&RoadMarking>,
) -> bool {
    !road_mesh_query.is_empty() && markings_query.is_empty()
}

/// Marker for road marking entities.
#[derive(Component)]
pub struct RoadMarking;

/// Configuration for road markings.
#[derive(Resource)]
pub struct MarkingsConfig {
    pub center_line_width: f32,
    pub dash_length: f32,
    pub gap_length: f32,
    pub marking_height: f32,
}

impl Default for MarkingsConfig {
    fn default() -> Self {
        Self {
            center_line_width: 0.3,
            dash_length: 3.0,
            gap_length: 2.0,
            marking_height: 0.15, // Slightly above road
        }
    }
}

/// Generate road markings.
fn generate_road_markings(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<MarkingsConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Generating road markings...");

    // Create terrain sampler
    let terrain = TerrainSampler::new(&terrain_config);

    // Yellow center line material
    let center_line_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.85, 0.3),
        perceptual_roughness: 0.7,
        ..default()
    });

    // White edge line material
    let edge_line_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.7,
        ..default()
    });

    for edge in road_graph.edges() {
        if edge.points.len() < 2 {
            continue;
        }

        // Skip edges that cross water - bridges don't get road markings
        if edge.crosses_water {
            continue;
        }

        // Only add markings to major roads
        let (add_center, add_edges) = match edge.road_type {
            RoadType::Highway => (true, true),
            RoadType::Major => (true, false),
            RoadType::Minor => (false, false),
            RoadType::Alley => (false, false),
        };

        if add_center {
            // Create dashed center line
            let dashes = create_dashed_line(
                &edge.points,
                config.center_line_width,
                config.dash_length,
                config.gap_length,
                config.marking_height,
                &terrain,
            );

            for dash_mesh in dashes {
                commands.spawn((
                    Mesh3d(meshes.add(dash_mesh)),
                    MeshMaterial3d(center_line_material.clone()),
                    Transform::IDENTITY,
                    RoadMarking,
                ));
            }
        }

        if add_edges {
            // Get road width for edge lines
            let road_width = match edge.road_type {
                RoadType::Highway => 12.0,
                RoadType::Major => 8.0,
                RoadType::Minor => 5.0,
                RoadType::Alley => 3.0,
            };

            // Left edge line
            let left_points = offset_polyline(&edge.points, road_width / 2.0 - 0.5);
            let left_dashes = create_dashed_line(
                &left_points,
                config.center_line_width * 0.8,
                config.dash_length * 2.0, // Longer dashes for edges
                config.gap_length,
                config.marking_height,
                &terrain,
            );

            for dash_mesh in left_dashes {
                commands.spawn((
                    Mesh3d(meshes.add(dash_mesh)),
                    MeshMaterial3d(edge_line_material.clone()),
                    Transform::IDENTITY,
                    RoadMarking,
                ));
            }

            // Right edge line
            let right_points = offset_polyline(&edge.points, -road_width / 2.0 + 0.5);
            let right_dashes = create_dashed_line(
                &right_points,
                config.center_line_width * 0.8,
                config.dash_length * 2.0,
                config.gap_length,
                config.marking_height,
                &terrain,
            );

            for dash_mesh in right_dashes {
                commands.spawn((
                    Mesh3d(meshes.add(dash_mesh)),
                    MeshMaterial3d(edge_line_material.clone()),
                    Transform::IDENTITY,
                    RoadMarking,
                ));
            }
        }
    }

    info!("Road markings generated");
}

/// Create dashed line meshes along a polyline.
fn create_dashed_line(
    points: &[Vec2],
    width: f32,
    dash_length: f32,
    gap_length: f32,
    height_offset: f32,
    terrain: &TerrainSampler,
) -> Vec<Mesh> {
    let mut meshes = Vec::new();
    let cycle_length = dash_length + gap_length;

    // Calculate total length and collect segments
    let mut segments: Vec<(Vec2, Vec2, f32)> = Vec::new(); // (start, end, start_distance)
    let mut total_dist = 0.0;

    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        let seg_length = start.distance(end);
        segments.push((start, end, total_dist));
        total_dist += seg_length;
    }

    // Generate dashes
    let mut current_dist = 0.0;
    while current_dist < total_dist {
        let dash_start = current_dist;
        let dash_end = (current_dist + dash_length).min(total_dist);

        if dash_end - dash_start > 0.5 {
            // Get start and end points
            if let (Some(start_pos), Some(end_pos)) = (
                point_at_distance(&segments, dash_start),
                point_at_distance(&segments, dash_end),
            ) {
                let mesh = create_dash_quad(start_pos, end_pos, width, height_offset, terrain);
                meshes.push(mesh);
            }
        }

        current_dist += cycle_length;
    }

    meshes
}

/// Get point at a specific distance along the polyline.
fn point_at_distance(segments: &[(Vec2, Vec2, f32)], distance: f32) -> Option<Vec2> {
    for &(start, end, seg_start) in segments {
        let seg_length = start.distance(end);
        // Skip degenerate segments
        if seg_length < 0.001 {
            continue;
        }
        let seg_end = seg_start + seg_length;

        if distance >= seg_start && distance <= seg_end {
            let t = (distance - seg_start) / seg_length;
            return Some(start.lerp(end, t));
        }
    }
    segments.last().map(|&(_, end, _)| end)
}

/// Create a single dash quad.
fn create_dash_quad(start: Vec2, end: Vec2, width: f32, height_offset: f32, terrain: &TerrainSampler) -> Mesh {
    let dir = (end - start).normalize_or_zero();
    let perp = Vec2::new(-dir.y, dir.x);
    let half_width = width / 2.0;

    let v0 = start + perp * half_width;
    let v1 = start - perp * half_width;
    let v2 = end + perp * half_width;
    let v3 = end - perp * half_width;

    // Sample terrain height at each vertex
    let h0 = terrain.sample(v0.x, v0.y) + height_offset;
    let h1 = terrain.sample(v1.x, v1.y) + height_offset;
    let h2 = terrain.sample(v2.x, v2.y) + height_offset;
    let h3 = terrain.sample(v3.x, v3.y) + height_offset;

    let vertices = vec![
        [v0.x, h0, v0.y],
        [v1.x, h1, v1.y],
        [v2.x, h2, v2.y],
        [v3.x, h3, v3.y],
    ];

    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];

    let uvs = vec![
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [1.0, 1.0],
    ];

    // CCW winding
    let indices = vec![0u32, 2, 1, 2, 3, 1];

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
        // Guard against NaN/Inf coordinates
        if !x.is_finite() || !z.is_finite() {
            return 0.0;
        }

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

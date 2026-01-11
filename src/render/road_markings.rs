//! Road lane markings - center lines, edge lines.
//!
//! All markings are batched into 2 meshes (yellow center, white edge) for performance.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use noise::{NoiseFn, Perlin};

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct RoadMarkingsPlugin;

impl Plugin for RoadMarkingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarkingsConfig>()
            .init_resource::<RoadMarkingsSpawned>()
            .add_systems(Update, generate_road_markings.run_if(should_generate_markings));
    }
}

/// Marker that road markings have been generated (prevents re-running).
#[derive(Resource, Default)]
pub struct RoadMarkingsSpawned(pub bool);

fn should_generate_markings(
    road_mesh_query: Query<&RoadMeshGenerated>,
    spawned: Res<RoadMarkingsSpawned>,
) -> bool {
    !road_mesh_query.is_empty() && !spawned.0
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

/// Batched mesh builder for road markings.
struct MarkingsMeshBuilder {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MarkingsMeshBuilder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Add a dash quad to the batch.
    fn add_dash(&mut self, start: Vec2, end: Vec2, width: f32, height_offset: f32, terrain: &TerrainSampler) {
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

        let base_index = self.vertices.len() as u32;

        // Add vertices
        self.vertices.push([v0.x, h0, v0.y]);
        self.vertices.push([v1.x, h1, v1.y]);
        self.vertices.push([v2.x, h2, v2.y]);
        self.vertices.push([v3.x, h3, v3.y]);

        // All normals point up
        self.normals.push([0.0, 1.0, 0.0]);
        self.normals.push([0.0, 1.0, 0.0]);
        self.normals.push([0.0, 1.0, 0.0]);
        self.normals.push([0.0, 1.0, 0.0]);

        // UVs
        self.uvs.push([0.0, 0.0]);
        self.uvs.push([0.0, 1.0]);
        self.uvs.push([1.0, 0.0]);
        self.uvs.push([1.0, 1.0]);

        // CCW winding triangles
        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);
        self.indices.push(base_index + 1);
    }

    /// Add dashed line along a polyline.
    fn add_dashed_line(
        &mut self,
        points: &[Vec2],
        width: f32,
        dash_length: f32,
        gap_length: f32,
        height_offset: f32,
        terrain: &TerrainSampler,
    ) {
        let cycle_length = dash_length + gap_length;

        // Calculate total length and collect segments
        let mut segments: Vec<(Vec2, Vec2, f32)> = Vec::new();
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
                if let (Some(start_pos), Some(end_pos)) = (
                    point_at_distance(&segments, dash_start),
                    point_at_distance(&segments, dash_end),
                ) {
                    self.add_dash(start_pos, end_pos, width, height_offset, terrain);
                }
            }

            current_dist += cycle_length;
        }
    }

    /// Build the final mesh (returns None if empty).
    fn build(self) -> Option<Mesh> {
        if self.vertices.is_empty() {
            return None;
        }

        Some(
            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
                .with_inserted_indices(Indices::U32(self.indices))
        )
    }
}

/// Generate road markings - batched into 2 meshes.
fn generate_road_markings(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<MarkingsConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<RoadMarkingsSpawned>,
) {
    info!("Generating road markings...");

    let terrain = TerrainSampler::new(&terrain_config);

    // Batched mesh builders
    let mut center_lines = MarkingsMeshBuilder::new();
    let mut edge_lines = MarkingsMeshBuilder::new();

    let mut center_count = 0;
    let mut edge_count = 0;

    for edge in road_graph.edges() {
        if edge.points.len() < 2 {
            continue;
        }

        // Skip edges that cross water - bridges don't get road markings
        if edge.crosses_water {
            continue;
        }

        // Determine which markings to add
        let (add_center, add_edges) = match edge.road_type {
            RoadType::Highway => (true, true),
            RoadType::Major => (true, false),
            RoadType::Minor => (false, false),
            RoadType::Alley => (false, false),
        };

        if add_center {
            center_lines.add_dashed_line(
                &edge.points,
                config.center_line_width,
                config.dash_length,
                config.gap_length,
                config.marking_height,
                &terrain,
            );
            center_count += 1;
        }

        if add_edges {
            let road_width = match edge.road_type {
                RoadType::Highway => 12.0,
                RoadType::Major => 8.0,
                RoadType::Minor => 5.0,
                RoadType::Alley => 3.0,
            };

            // Left edge line
            let left_points = offset_polyline(&edge.points, road_width / 2.0 - 0.5);
            edge_lines.add_dashed_line(
                &left_points,
                config.center_line_width * 0.8,
                config.dash_length * 2.0,
                config.gap_length,
                config.marking_height,
                &terrain,
            );

            // Right edge line
            let right_points = offset_polyline(&edge.points, -road_width / 2.0 + 0.5);
            edge_lines.add_dashed_line(
                &right_points,
                config.center_line_width * 0.8,
                config.dash_length * 2.0,
                config.gap_length,
                config.marking_height,
                &terrain,
            );

            edge_count += 1;
        }
    }

    // Spawn center line mesh (yellow)
    if let Some(center_mesh) = center_lines.build() {
        let center_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.85, 0.3),
            perceptual_roughness: 0.7,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(center_mesh)),
            MeshMaterial3d(center_material),
            Transform::IDENTITY,
            RoadMarking,
        ));
    }

    // Spawn edge line mesh (white)
    if let Some(edge_mesh) = edge_lines.build() {
        let edge_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.95),
            perceptual_roughness: 0.7,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(edge_mesh)),
            MeshMaterial3d(edge_material),
            Transform::IDENTITY,
            RoadMarking,
        ));
    }

    spawned.0 = true;
    info!(
        "Road markings generated: {} center lines, {} edge lines (2 batched meshes)",
        center_count, edge_count
    );
}

/// Get point at a specific distance along the polyline.
fn point_at_distance(segments: &[(Vec2, Vec2, f32)], distance: f32) -> Option<Vec2> {
    for &(start, end, seg_start) in segments {
        let seg_length = start.distance(end);
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

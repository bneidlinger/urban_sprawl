//! Crosswalk generation at intersections.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};
use petgraph::graph::NodeIndex;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct CrosswalksPlugin;

impl Plugin for CrosswalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrosswalkConfig>()
            .add_systems(Update, spawn_crosswalks.run_if(should_spawn_crosswalks));
    }
}

fn should_spawn_crosswalks(
    road_mesh_query: Query<&RoadMeshGenerated>,
    crosswalk_query: Query<&Crosswalk>,
) -> bool {
    !road_mesh_query.is_empty() && crosswalk_query.is_empty()
}

#[derive(Component)]
pub struct Crosswalk;

#[derive(Resource)]
pub struct CrosswalkConfig {
    pub stripe_width: f32,
    pub stripe_length: f32,
    pub stripe_spacing: f32,
    pub num_stripes: usize,
    pub height_offset: f32,
}

impl Default for CrosswalkConfig {
    fn default() -> Self {
        Self {
            stripe_width: 0.5,
            stripe_length: 3.0,
            stripe_spacing: 0.6,
            num_stripes: 6,
            height_offset: 0.12,
        }
    }
}

fn spawn_crosswalks(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<CrosswalkConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning crosswalks...");

    // Create terrain sampler
    let terrain = TerrainSampler::new(&terrain_config);

    // White paint material
    let crosswalk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.7,
        ..default()
    });

    let mut crosswalk_count = 0;

    // Find intersections (nodes with 3+ connections)
    for (node_idx, node) in road_graph.nodes() {
        let neighbors: Vec<NodeIndex> = road_graph.graph.neighbors(node_idx).collect();

        if neighbors.len() < 3 {
            continue;
        }

        // Get directions and widths to neighboring roads
        for neighbor_idx in &neighbors {
            if let Some(neighbor_node) = road_graph.graph.node_weight(*neighbor_idx) {
                // Get road type for this edge
                let road_width = if let Some(edge_idx) = road_graph.graph.find_edge(node_idx, *neighbor_idx) {
                    if let Some(edge) = road_graph.graph.edge_weight(edge_idx) {
                        match edge.road_type {
                            RoadType::Highway => continue, // No crosswalks on highways
                            RoadType::Major => 8.0,
                            RoadType::Minor => 5.0,
                            RoadType::Alley => continue, // No crosswalks on alleys
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                let dir = (neighbor_node.position - node.position).normalize_or_zero();

                // Position crosswalk slightly away from intersection center
                let crosswalk_distance = road_width * 0.8;
                let crosswalk_center = node.position + dir * crosswalk_distance;

                // Sample terrain height at crosswalk center
                let terrain_height = terrain.sample(crosswalk_center.x, crosswalk_center.y);

                // Create crosswalk mesh (series of stripes perpendicular to road)
                let mesh = create_crosswalk_mesh(&config, road_width);

                // Calculate rotation to align with road
                let angle = dir.y.atan2(dir.x);

                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(crosswalk_material.clone()),
                    Transform::from_xyz(crosswalk_center.x, terrain_height + config.height_offset, crosswalk_center.y)
                        .with_rotation(Quat::from_rotation_y(-angle)),
                    Crosswalk,
                ));

                crosswalk_count += 1;
            }
        }
    }

    info!("Spawned {} crosswalks", crosswalk_count);
}

/// Create a crosswalk mesh with parallel stripes.
fn create_crosswalk_mesh(config: &CrosswalkConfig, road_width: f32) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let half_length = config.stripe_length / 2.0;
    let total_width = (config.num_stripes as f32) * config.stripe_width
        + (config.num_stripes as f32 - 1.0) * config.stripe_spacing;
    let start_offset = -total_width / 2.0;

    for i in 0..config.num_stripes {
        let stripe_center = start_offset + (i as f32) * (config.stripe_width + config.stripe_spacing) + config.stripe_width / 2.0;

        let half_stripe = config.stripe_width / 2.0;
        let base_idx = vertices.len() as u32;

        // Four corners of the stripe (oriented along X axis, stripe runs along Z)
        // Left-back
        vertices.push([stripe_center - half_stripe, 0.0, -half_length]);
        // Right-back
        vertices.push([stripe_center + half_stripe, 0.0, -half_length]);
        // Right-front
        vertices.push([stripe_center + half_stripe, 0.0, half_length]);
        // Left-front
        vertices.push([stripe_center - half_stripe, 0.0, half_length]);

        for _ in 0..4 {
            normals.push([0.0, 1.0, 0.0]);
        }

        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 0.0]);
        uvs.push([1.0, 1.0]);
        uvs.push([0.0, 1.0]);

        // Two triangles (CCW winding)
        indices.push(base_idx);
        indices.push(base_idx + 2);
        indices.push(base_idx + 1);
        indices.push(base_idx);
        indices.push(base_idx + 3);
        indices.push(base_idx + 2);
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
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

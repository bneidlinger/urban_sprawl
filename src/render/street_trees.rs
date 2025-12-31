//! Street tree generation along sidewalks.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct StreetTreesPlugin;

impl Plugin for StreetTreesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetTreeConfig>()
            .add_systems(Update, spawn_street_trees.run_if(should_spawn_trees));
    }
}

fn should_spawn_trees(
    road_mesh_query: Query<&RoadMeshGenerated>,
    tree_query: Query<&StreetTree>,
) -> bool {
    !road_mesh_query.is_empty() && tree_query.is_empty()
}

/// Marker component for street trees.
#[derive(Component)]
pub struct StreetTree;

/// Configuration for street tree placement.
#[derive(Resource)]
pub struct StreetTreeConfig {
    pub spacing: f32,
    pub offset_from_road: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub trunk_radius: f32,
    pub foliage_min: f32,
    pub foliage_max: f32,
    pub seed: u64,
}

impl Default for StreetTreeConfig {
    fn default() -> Self {
        Self {
            spacing: 25.0,
            offset_from_road: 4.5,
            min_height: 5.0,
            max_height: 10.0,
            trunk_radius: 0.25,
            foliage_min: 2.0,
            foliage_max: 3.5,
            seed: 54321,
        }
    }
}

// Foliage color palette
const FOLIAGE_COLORS: &[(f32, f32, f32)] = &[
    (0.2, 0.45, 0.15),   // Dark green
    (0.25, 0.5, 0.2),    // Medium green
    (0.18, 0.42, 0.12),  // Forest green
];

fn spawn_street_trees(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<StreetTreeConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning street trees...");

    let terrain = TerrainSampler::new(&terrain_config);
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Create trunk mesh (unit cylinder, scaled per tree)
    let trunk_mesh = meshes.add(Cylinder::new(config.trunk_radius, 1.0));
    let foliage_mesh = meshes.add(Sphere::new(1.0));

    // Trunk material (brown bark)
    let trunk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Pre-create foliage materials
    let foliage_materials: Vec<_> = FOLIAGE_COLORS
        .iter()
        .map(|&(r, g, b)| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                perceptual_roughness: 0.8,
                ..default()
            })
        })
        .collect();

    let mut tree_count = 0;

    for edge in road_graph.edges() {
        // Only place trees on Major and Minor roads
        let road_width = match edge.road_type {
            RoadType::Highway => continue,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => continue,
        };

        if edge.points.len() < 2 {
            continue;
        }

        // Calculate offset from road center to sidewalk
        let tree_offset = road_width / 2.0 + config.offset_from_road;

        let mut accumulated_dist = config.spacing / 2.0; // Start offset
        let mut segment_start_dist = 0.0;

        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let segment_length = start.distance(end);
            let segment_end_dist = segment_start_dist + segment_length;

            let dir = (end - start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);

            while accumulated_dist < segment_end_dist {
                let t = (accumulated_dist - segment_start_dist) / segment_length;
                let pos = start.lerp(end, t);

                // Alternate sides
                let side = if tree_count % 2 == 0 { 1.0 } else { -1.0 };
                let tree_pos = pos + perp * tree_offset * side;

                // Random tree dimensions
                let tree_height = rng.gen_range(config.min_height..config.max_height);
                let foliage_size = rng.gen_range(config.foliage_min..config.foliage_max);

                // Sample terrain height
                let terrain_height = terrain.sample(tree_pos.x, tree_pos.y);

                // Random foliage color
                let foliage_mat = foliage_materials[rng.gen_range(0..foliage_materials.len())].clone();

                // Spawn trunk
                let trunk_y = terrain_height + tree_height / 2.0;
                commands.spawn((
                    Mesh3d(trunk_mesh.clone()),
                    MeshMaterial3d(trunk_material.clone()),
                    Transform::from_xyz(tree_pos.x, trunk_y, tree_pos.y)
                        .with_scale(Vec3::new(1.0, tree_height, 1.0)),
                    StreetTree,
                ));

                // Spawn foliage (sphere on top of trunk)
                let foliage_y = terrain_height + tree_height + foliage_size * 0.3;
                commands.spawn((
                    Mesh3d(foliage_mesh.clone()),
                    MeshMaterial3d(foliage_mat),
                    Transform::from_xyz(tree_pos.x, foliage_y, tree_pos.y)
                        .with_scale(Vec3::splat(foliage_size)),
                    StreetTree,
                ));

                tree_count += 1;
                accumulated_dist += config.spacing;
            }

            segment_start_dist = segment_end_dist;
        }
    }

    info!("Spawned {} street trees", tree_count);
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

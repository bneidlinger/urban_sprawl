//! Street-level details: manholes, storm drains, bollards.
//!
//! Small urban details that add realism to the city streets.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin};
use petgraph::graph::NodeIndex;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::crosswalks::Crosswalk;
use crate::render::gpu_culling::GpuCullable;
use crate::render::instancing::TerrainConfig;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct StreetDetailsPlugin;

impl Plugin for StreetDetailsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetDetailsConfig>()
            .init_resource::<StreetDetailsSpawned>()
            .init_resource::<BollardsSpawned>()
            .add_systems(Update, spawn_street_details.run_if(should_spawn_details))
            .add_systems(Update, spawn_bollards.run_if(should_spawn_bollards));
    }
}

/// Marker to prevent respawning.
#[derive(Resource, Default)]
pub struct StreetDetailsSpawned(pub bool);

/// Marker to prevent bollards from respawning.
#[derive(Resource, Default)]
pub struct BollardsSpawned(pub bool);

fn should_spawn_details(
    road_mesh_query: Query<&RoadMeshGenerated>,
    spawned: Res<StreetDetailsSpawned>,
) -> bool {
    !road_mesh_query.is_empty() && !spawned.0
}

fn should_spawn_bollards(
    crosswalk_query: Query<&Crosswalk>,
    spawned: Res<BollardsSpawned>,
) -> bool {
    !crosswalk_query.is_empty() && !spawned.0
}

/// Component for manhole covers.
#[derive(Component)]
pub struct Manhole;

/// Component for storm drain grates.
#[derive(Component)]
pub struct StormDrain;

/// Component for bollards.
#[derive(Component)]
pub struct Bollard;

/// Configuration for street details.
#[derive(Resource)]
pub struct StreetDetailsConfig {
    pub seed: u64,
    /// Spacing between manholes along roads
    pub manhole_spacing: f32,
    /// Manhole cover diameter
    pub manhole_diameter: f32,
    /// Probability of manhole at each spacing point
    pub manhole_probability: f32,
    /// Storm drain grate width
    pub drain_width: f32,
    /// Storm drain grate length
    pub drain_length: f32,
    /// Bollard height
    pub bollard_height: f32,
    /// Bollard radius
    pub bollard_radius: f32,
    /// Number of bollards per crosswalk side
    pub bollards_per_crosswalk: usize,
}

impl Default for StreetDetailsConfig {
    fn default() -> Self {
        Self {
            seed: 54321,
            manhole_spacing: 40.0,      // Every 40m
            manhole_diameter: 0.8,       // 80cm diameter
            manhole_probability: 0.7,    // 70% chance
            drain_width: 0.5,            // 50cm wide
            drain_length: 0.15,          // 15cm deep
            bollard_height: 0.9,         // 90cm tall
            bollard_radius: 0.1,         // 10cm radius
            bollards_per_crosswalk: 2,   // 2 per side
        }
    }
}

fn spawn_street_details(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<StreetDetailsConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<StreetDetailsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning street details...");

    let mut rng = StdRng::seed_from_u64(config.seed);
    let terrain = TerrainSampler::new(&terrain_config);

    // Materials
    let manhole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.27),
        perceptual_roughness: 0.8,
        metallic: 0.6,
        ..default()
    });

    let drain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.17),
        perceptual_roughness: 0.9,
        metallic: 0.3,
        ..default()
    });

    // Meshes
    let manhole_mesh = meshes.add(create_manhole_mesh(config.manhole_diameter));
    let drain_mesh = meshes.add(create_drain_grate_mesh(config.drain_width, config.drain_length));

    let mut manhole_count = 0;
    let mut drain_count = 0;

    // Spawn manholes along roads
    for edge in road_graph.edges() {
        let road_width = match edge.road_type {
            RoadType::Highway => continue, // No manholes on highways
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => continue,
        };

        if edge.points.len() < 2 {
            continue;
        }

        // Manholes slightly off-center on the road
        let manhole_offset = road_width * 0.25;

        let mut accumulated_dist = rng.gen_range(5.0..config.manhole_spacing);
        let mut segment_start_dist = 0.0;

        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let segment_length = start.distance(end);
            let segment_end_dist = segment_start_dist + segment_length;

            let dir = (end - start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);

            while accumulated_dist < segment_end_dist {
                if rng.gen::<f32>() < config.manhole_probability {
                    let t = (accumulated_dist - segment_start_dist) / segment_length;
                    let pos = start.lerp(end, t);

                    // Alternate sides slightly
                    let side = if manhole_count % 2 == 0 { 1.0 } else { -1.0 };
                    let manhole_pos = pos + perp * manhole_offset * side;
                    let height = terrain.sample(manhole_pos.x, manhole_pos.y) + 0.02;

                    // Random rotation for variety
                    let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

                    commands.spawn((
                        Mesh3d(manhole_mesh.clone()),
                        MeshMaterial3d(manhole_material.clone()),
                        Transform::from_xyz(manhole_pos.x, height, manhole_pos.y)
                            .with_rotation(rotation),
                        Manhole,
                        GpuCullable::new(config.manhole_diameter / 2.0),
                    ));

                    manhole_count += 1;
                }

                accumulated_dist += config.manhole_spacing;
            }

            segment_start_dist = segment_end_dist;
        }
    }

    // Spawn storm drains at intersections
    for (node_idx, node) in road_graph.nodes() {
        let neighbors: Vec<NodeIndex> = road_graph.graph.neighbors(node_idx).collect();

        // Only at real intersections (3+ connections)
        if neighbors.len() < 3 {
            continue;
        }

        // Place drains at each corner of the intersection
        for (i, neighbor_idx) in neighbors.iter().enumerate() {
            if let Some(neighbor_node) = road_graph.graph.node_weight(*neighbor_idx) {
                let dir = (neighbor_node.position - node.position).normalize_or_zero();
                let perp = Vec2::new(-dir.y, dir.x);

                // Get road width
                let road_width = if let Some(edge_idx) = road_graph.graph.find_edge(node_idx, *neighbor_idx) {
                    if let Some(edge) = road_graph.graph.edge_weight(edge_idx) {
                        match edge.road_type {
                            RoadType::Highway => continue,
                            RoadType::Major => 8.0,
                            RoadType::Minor => 5.0,
                            RoadType::Alley => continue,
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Place drain at curb corner
                let drain_distance = road_width * 0.6;
                let drain_side = if i % 2 == 0 { 1.0 } else { -1.0 };
                let drain_pos = node.position + dir * drain_distance + perp * (road_width / 2.0 + 1.0) * drain_side;
                let height = terrain.sample(drain_pos.x, drain_pos.y) + 0.01;

                // Rotate to face road
                let angle = dir.y.atan2(dir.x);

                commands.spawn((
                    Mesh3d(drain_mesh.clone()),
                    MeshMaterial3d(drain_material.clone()),
                    Transform::from_xyz(drain_pos.x, height, drain_pos.y)
                        .with_rotation(Quat::from_rotation_y(-angle)),
                    StormDrain,
                    GpuCullable::new(config.drain_width),
                ));

                drain_count += 1;
            }
        }
    }

    info!(
        "Spawned street details: {} manholes, {} storm drains",
        manhole_count, drain_count
    );
}

/// Spawn bollards near crosswalks (separate system that runs after crosswalks spawn).
fn spawn_bollards(
    mut commands: Commands,
    config: Res<StreetDetailsConfig>,
    terrain_config: Res<TerrainConfig>,
    crosswalk_query: Query<&Transform, With<Crosswalk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BollardsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning bollards near crosswalks...");

    let mut rng = StdRng::seed_from_u64(config.seed + 100);
    let terrain = TerrainSampler::new(&terrain_config);

    let bollard_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.5,
        metallic: 0.7,
        ..default()
    });

    let bollard_stripe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });

    let bollard_mesh = meshes.add(Cylinder::new(config.bollard_radius, config.bollard_height));
    let bollard_cap_mesh = meshes.add(Sphere::new(config.bollard_radius * 1.2));
    let bollard_stripe_mesh = meshes.add(Cylinder::new(config.bollard_radius * 1.05, 0.05));

    let mut bollard_count = 0;

    // Get unique crosswalk positions
    let mut crosswalk_positions: Vec<Vec3> = Vec::new();
    for transform in crosswalk_query.iter() {
        let pos = transform.translation;
        // Check if we already have a nearby crosswalk
        let is_duplicate = crosswalk_positions.iter().any(|existing| {
            existing.distance(pos) < 3.0
        });
        if !is_duplicate {
            crosswalk_positions.push(pos);
        }
    }

    // Only place bollards at some crosswalks to avoid clutter
    for crosswalk_pos in crosswalk_positions.iter() {
        if rng.gen::<f32>() > 0.4 {
            continue; // 40% of crosswalks get bollards
        }

        // Place bollards on sidewalk sides
        for side in [-1.0_f32, 1.0] {
            for j in 0..config.bollards_per_crosswalk {
                let offset_along = (j as f32 - 0.5) * 2.0;
                let offset_perp = side * 4.5; // On sidewalk

                let bollard_pos = Vec3::new(
                    crosswalk_pos.x + offset_along,
                    0.0,
                    crosswalk_pos.z + offset_perp,
                );
                let height = terrain.sample(bollard_pos.x, bollard_pos.z);

                // Main bollard body
                commands.spawn((
                    Mesh3d(bollard_mesh.clone()),
                    MeshMaterial3d(bollard_material.clone()),
                    Transform::from_xyz(bollard_pos.x, height + config.bollard_height / 2.0, bollard_pos.z),
                    Bollard,
                    GpuCullable::new(config.bollard_height),
                ));

                // Rounded cap
                commands.spawn((
                    Mesh3d(bollard_cap_mesh.clone()),
                    MeshMaterial3d(bollard_material.clone()),
                    Transform::from_xyz(bollard_pos.x, height + config.bollard_height, bollard_pos.z),
                    Bollard,
                    GpuCullable::new(config.bollard_radius * 1.2),
                ));

                // Yellow reflective stripe
                commands.spawn((
                    Mesh3d(bollard_stripe_mesh.clone()),
                    MeshMaterial3d(bollard_stripe_material.clone()),
                    Transform::from_xyz(bollard_pos.x, height + config.bollard_height * 0.7, bollard_pos.z),
                    Bollard,
                    GpuCullable::new(config.bollard_radius * 1.05),
                ));

                bollard_count += 1;
            }
        }
    }

    info!("Spawned {} bollards near crosswalks", bollard_count);
}

/// Create a circular manhole cover mesh with a pattern.
fn create_manhole_mesh(diameter: f32) -> Mesh {
    let radius = diameter / 2.0;
    let segments = 24;

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Center vertex
    vertices.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Outer ring
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        vertices.push([x, 0.0, z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([(angle.cos() + 1.0) / 2.0, (angle.sin() + 1.0) / 2.0]);
    }

    // Inner ring (for visual pattern)
    let inner_radius = radius * 0.85;
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * inner_radius;
        let z = angle.sin() * inner_radius;

        vertices.push([x, 0.005, z]); // Slightly raised
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([(angle.cos() * 0.85 + 1.0) / 2.0, (angle.sin() * 0.85 + 1.0) / 2.0]);
    }

    // Outer triangles
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push((next + 1) as u32);
    }

    // Inner ring triangles (connecting inner and outer rings)
    let inner_start = segments + 1;
    for i in 0..segments {
        let next = (i + 1) % segments;
        let outer_curr = (i + 1) as u32;
        let outer_next = (next + 1) as u32;
        let inner_curr = (inner_start + i) as u32;
        let inner_next = (inner_start + next) as u32;

        // Two triangles per segment
        indices.push(outer_curr);
        indices.push(inner_curr);
        indices.push(outer_next);

        indices.push(outer_next);
        indices.push(inner_curr);
        indices.push(inner_next);
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a storm drain grate mesh with slots.
fn create_drain_grate_mesh(width: f32, slot_width: f32) -> Mesh {
    let length = width * 1.5;
    let num_slots = 5;
    let frame_thickness = 0.04;
    let slot_spacing = (length - frame_thickness * 2.0) / (num_slots as f32 + 0.5);

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Outer frame
    let half_w = width / 2.0;
    let half_l = length / 2.0;

    // Top surface (frame only, with slots cut out)
    // We'll create the frame as 4 edge strips plus the bars between slots

    // Add frame edges
    let add_quad = |verts: &mut Vec<[f32; 3]>, norms: &mut Vec<[f32; 3]>, uv: &mut Vec<[f32; 2]>,
                   inds: &mut Vec<u32>, corners: [[f32; 3]; 4]| {
        let base = verts.len() as u32;
        for corner in corners {
            verts.push(corner);
            norms.push([0.0, 1.0, 0.0]);
            uv.push([0.0, 0.0]);
        }
        inds.extend_from_slice(&[base, base + 2, base + 1, base + 2, base + 3, base + 1]);
    };

    // Left edge
    add_quad(&mut vertices, &mut normals, &mut uvs, &mut indices, [
        [-half_w, 0.0, -half_l],
        [-half_w + frame_thickness, 0.0, -half_l],
        [-half_w, 0.0, half_l],
        [-half_w + frame_thickness, 0.0, half_l],
    ]);

    // Right edge
    add_quad(&mut vertices, &mut normals, &mut uvs, &mut indices, [
        [half_w - frame_thickness, 0.0, -half_l],
        [half_w, 0.0, -half_l],
        [half_w - frame_thickness, 0.0, half_l],
        [half_w, 0.0, half_l],
    ]);

    // Top edge
    add_quad(&mut vertices, &mut normals, &mut uvs, &mut indices, [
        [-half_w + frame_thickness, 0.0, half_l - frame_thickness],
        [half_w - frame_thickness, 0.0, half_l - frame_thickness],
        [-half_w + frame_thickness, 0.0, half_l],
        [half_w - frame_thickness, 0.0, half_l],
    ]);

    // Bottom edge
    add_quad(&mut vertices, &mut normals, &mut uvs, &mut indices, [
        [-half_w + frame_thickness, 0.0, -half_l],
        [half_w - frame_thickness, 0.0, -half_l],
        [-half_w + frame_thickness, 0.0, -half_l + frame_thickness],
        [half_w - frame_thickness, 0.0, -half_l + frame_thickness],
    ]);

    // Horizontal bars (between slots)
    let bar_width = slot_spacing * 0.3;
    for i in 0..=num_slots {
        let z = -half_l + frame_thickness + (i as f32 + 0.25) * slot_spacing;
        add_quad(&mut vertices, &mut normals, &mut uvs, &mut indices, [
            [-half_w + frame_thickness, 0.0, z],
            [half_w - frame_thickness, 0.0, z],
            [-half_w + frame_thickness, 0.0, z + bar_width],
            [half_w - frame_thickness, 0.0, z + bar_width],
        ]);
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

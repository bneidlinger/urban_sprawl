//! Elevated railway tracks through the city.
//!
//! Creates elevated rail infrastructure with tracks, support pillars,
//! and station platforms along a path through commercial areas.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::procgen::road_generator::RoadsGenerated;
use crate::procgen::roads::{RoadGraph, RoadType};

pub struct ElevatedRailPlugin;

impl Plugin for ElevatedRailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ElevatedRailConfig>()
            .init_resource::<ElevatedRailSpawned>()
            .init_resource::<RailLine>()
            .add_systems(Update, spawn_elevated_rail.run_if(should_spawn_rail));
    }
}

#[derive(Resource, Default)]
pub struct ElevatedRailSpawned(pub bool);

fn should_spawn_rail(roads_generated: Res<RoadsGenerated>, spawned: Res<ElevatedRailSpawned>) -> bool {
    roads_generated.0 && !spawned.0
}

/// Configuration for elevated rail.
#[derive(Resource)]
pub struct ElevatedRailConfig {
    pub seed: u64,
    /// Height of track bed above ground.
    pub track_height: f32,
    /// Width of the track bed/viaduct.
    pub track_width: f32,
    /// Distance between support pillars.
    pub pillar_spacing: f32,
    /// Width of support pillars.
    pub pillar_width: f32,
    /// Rail gauge (distance between rails).
    pub rail_gauge: f32,
    /// Station platform length.
    pub station_length: f32,
    /// Station platform width (additional to track).
    pub platform_width: f32,
}

impl Default for ElevatedRailConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            track_height: 8.0,
            track_width: 4.0,
            pillar_spacing: 20.0,
            pillar_width: 1.5,
            rail_gauge: 1.435, // Standard gauge
            station_length: 40.0,
            platform_width: 3.0,
        }
    }
}

/// The rail line path through the city.
#[derive(Resource, Default)]
pub struct RailLine {
    /// Waypoints defining the elevated rail route.
    pub waypoints: Vec<Vec2>,
    /// Station positions along the line (indices into waypoints).
    pub stations: Vec<usize>,
}

/// Marker component for elevated rail track segment.
#[derive(Component)]
pub struct ElevatedRailTrack {
    pub segment_index: usize,
}

/// Marker component for rail support pillar.
#[derive(Component)]
pub struct RailPillar;

/// Marker component for rail station platform.
#[derive(Component)]
pub struct RailStation {
    pub station_id: usize,
}

fn spawn_elevated_rail(
    mut commands: Commands,
    config: Res<ElevatedRailConfig>,
    road_graph: Res<RoadGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<ElevatedRailSpawned>,
    mut rail_line: ResMut<RailLine>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Generate rail line path by finding a route through major roads
    let waypoints = generate_rail_path(&road_graph, &mut rng);
    if waypoints.len() < 2 {
        info!("Not enough waypoints for elevated rail");
        return;
    }

    // Determine station locations (every ~5 segments)
    let stations: Vec<usize> = (0..waypoints.len())
        .step_by(5)
        .filter(|&i| i > 0 && i < waypoints.len() - 1)
        .collect();

    rail_line.waypoints = waypoints.clone();
    rail_line.stations = stations.clone();

    // Materials
    let concrete_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.52),
        perceptual_roughness: 0.9,
        ..default()
    });

    let rail_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.28, 0.25),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let platform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.45, 0.48),
        perceptual_roughness: 0.8,
        ..default()
    });

    let safety_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.8, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });

    // Meshes
    let pillar_mesh = meshes.add(create_pillar_mesh(config.pillar_width, config.track_height));
    let rail_mesh = meshes.add(Cuboid::new(0.07, 0.1, 1.0)); // 1m rail segment
    let tie_mesh = meshes.add(Cuboid::new(config.rail_gauge + 0.6, 0.1, 0.15));

    // Spawn track segments
    for i in 0..waypoints.len() - 1 {
        let start = waypoints[i];
        let end = waypoints[i + 1];
        let dir = (end - start).normalize_or_zero();
        let length = start.distance(end);
        let mid = (start + end) / 2.0;
        let angle = dir.y.atan2(dir.x);

        // Track bed (viaduct)
        let bed_mesh = meshes.add(create_track_bed_mesh(length, config.track_width, 0.5));
        commands.spawn((
            Mesh3d(bed_mesh),
            MeshMaterial3d(concrete_material.clone()),
            Transform::from_xyz(mid.x, config.track_height, mid.y)
                .with_rotation(Quat::from_rotation_y(-angle)),
            ElevatedRailTrack { segment_index: i },
        ));

        // Rails on top of bed
        let rail_count = (length / 1.0).ceil() as i32;
        for r in 0..rail_count {
            let t = r as f32 / rail_count as f32;
            let pos = start.lerp(end, t);

            // Left rail
            commands.spawn((
                Mesh3d(rail_mesh.clone()),
                MeshMaterial3d(rail_material.clone()),
                Transform::from_xyz(
                    pos.x - dir.y * config.rail_gauge / 2.0,
                    config.track_height + 0.3,
                    pos.y + dir.x * config.rail_gauge / 2.0,
                )
                .with_rotation(Quat::from_rotation_y(-angle)),
            ));

            // Right rail
            commands.spawn((
                Mesh3d(rail_mesh.clone()),
                MeshMaterial3d(rail_material.clone()),
                Transform::from_xyz(
                    pos.x + dir.y * config.rail_gauge / 2.0,
                    config.track_height + 0.3,
                    pos.y - dir.x * config.rail_gauge / 2.0,
                )
                .with_rotation(Quat::from_rotation_y(-angle)),
            ));

            // Cross ties
            commands.spawn((
                Mesh3d(tie_mesh.clone()),
                MeshMaterial3d(concrete_material.clone()),
                Transform::from_xyz(pos.x, config.track_height + 0.25, pos.y)
                    .with_rotation(Quat::from_rotation_y(-angle)),
            ));
        }

        // Support pillars
        let pillar_count = (length / config.pillar_spacing).ceil() as i32;
        for p in 0..=pillar_count {
            let t = p as f32 / pillar_count as f32;
            let pos = start.lerp(end, t);

            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(concrete_material.clone()),
                Transform::from_xyz(pos.x, config.track_height / 2.0, pos.y),
                RailPillar,
            ));
        }
    }

    // Spawn stations
    for (station_id, &wp_idx) in stations.iter().enumerate() {
        let pos = waypoints[wp_idx];

        // Get direction from neighboring waypoints
        let dir = if wp_idx > 0 && wp_idx < waypoints.len() - 1 {
            (waypoints[wp_idx + 1] - waypoints[wp_idx - 1]).normalize_or_zero()
        } else {
            Vec2::X
        };
        let perp = Vec2::new(-dir.y, dir.x);
        let angle = dir.y.atan2(dir.x);

        // Platform mesh (both sides)
        let platform_mesh = meshes.add(create_platform_mesh(
            config.station_length,
            config.platform_width,
            0.3,
        ));

        // Left platform
        let left_offset = perp * (config.track_width / 2.0 + config.platform_width / 2.0);
        commands.spawn((
            Mesh3d(platform_mesh.clone()),
            MeshMaterial3d(platform_material.clone()),
            Transform::from_xyz(
                pos.x + left_offset.x,
                config.track_height + 0.15,
                pos.y + left_offset.y,
            )
            .with_rotation(Quat::from_rotation_y(-angle)),
            RailStation { station_id },
        ));

        // Right platform
        let right_offset = perp * -(config.track_width / 2.0 + config.platform_width / 2.0);
        commands.spawn((
            Mesh3d(platform_mesh.clone()),
            MeshMaterial3d(platform_material.clone()),
            Transform::from_xyz(
                pos.x + right_offset.x,
                config.track_height + 0.15,
                pos.y + right_offset.y,
            )
            .with_rotation(Quat::from_rotation_y(-angle)),
            RailStation { station_id },
        ));

        // Safety line (yellow edge stripe)
        let safety_mesh = meshes.add(Cuboid::new(config.station_length, 0.02, 0.2));
        for side in [-1.0, 1.0] {
            let offset = perp * side * (config.track_width / 2.0 + 0.3);
            commands.spawn((
                Mesh3d(safety_mesh.clone()),
                MeshMaterial3d(safety_material.clone()),
                Transform::from_xyz(pos.x + offset.x, config.track_height + 0.31, pos.y + offset.y)
                    .with_rotation(Quat::from_rotation_y(-angle)),
            ));
        }

        // Canopy/shelter over platform
        let canopy_mesh = meshes.add(Cuboid::new(config.station_length * 0.8, 0.15, config.platform_width));
        for side in [-1.0, 1.0] {
            let offset = perp * side * (config.track_width / 2.0 + config.platform_width / 2.0);
            commands.spawn((
                Mesh3d(canopy_mesh.clone()),
                MeshMaterial3d(concrete_material.clone()),
                Transform::from_xyz(
                    pos.x + offset.x,
                    config.track_height + 3.5,
                    pos.y + offset.y,
                )
                .with_rotation(Quat::from_rotation_y(-angle)),
            ));
        }
    }

    info!(
        "Spawned elevated rail with {} segments, {} pillars, {} stations",
        waypoints.len() - 1,
        ((waypoints.len() - 1) as f32 * 2.0) as i32,
        stations.len()
    );
}

/// Generate a rail path through the city following major roads.
fn generate_rail_path(road_graph: &RoadGraph, rng: &mut StdRng) -> Vec<Vec2> {
    // Collect major road endpoints
    let mut major_nodes: Vec<Vec2> = road_graph
        .edges()
        .filter(|e| matches!(e.road_type, RoadType::Major | RoadType::Highway))
        .flat_map(|e| e.points.iter().cloned())
        .collect();

    if major_nodes.is_empty() {
        return Vec::new();
    }

    // Deduplicate nearby points
    major_nodes.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    let mut unique_nodes: Vec<Vec2> = Vec::new();
    for node in major_nodes {
        if unique_nodes.iter().all(|n| n.distance(node) > 30.0) {
            unique_nodes.push(node);
        }
    }

    if unique_nodes.len() < 3 {
        return Vec::new();
    }

    // Sort by x coordinate to create a roughly east-west line
    unique_nodes.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    // Take a subset of points to form the rail line
    let max_waypoints = 15.min(unique_nodes.len());
    let step = unique_nodes.len() / max_waypoints;

    let mut waypoints: Vec<Vec2> = unique_nodes
        .iter()
        .step_by(step.max(1))
        .take(max_waypoints)
        .cloned()
        .collect();

    // Add some offset variation to make it more interesting
    for wp in waypoints.iter_mut() {
        wp.y += rng.gen_range(-10.0..10.0);
    }

    waypoints
}

/// Create a pillar mesh (tapered rectangular column).
fn create_pillar_mesh(width: f32, height: f32) -> Mesh {
    let hw = width / 2.0;
    let hh = height / 2.0;
    let taper = 0.8; // Top is 80% of bottom width

    let positions = vec![
        // Bottom face
        [-hw, -hh, -hw],
        [hw, -hh, -hw],
        [hw, -hh, hw],
        [-hw, -hh, hw],
        // Top face (tapered)
        [-hw * taper, hh, -hw * taper],
        [hw * taper, hh, -hw * taper],
        [hw * taper, hh, hw * taper],
        [-hw * taper, hh, hw * taper],
    ];

    let indices = vec![
        // Bottom
        0, 2, 1, 0, 3, 2, // Top
        4, 5, 6, 4, 6, 7, // Front
        0, 1, 5, 0, 5, 4, // Back
        2, 3, 7, 2, 7, 6, // Left
        0, 4, 7, 0, 7, 3, // Right
        1, 2, 6, 1, 6, 5,
    ];

    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            let n = Vec3::from_array(*p).normalize_or_zero();
            [n.x, n.y.max(0.1), n.z]
        })
        .collect();

    let uvs: Vec<[f32; 2]> = positions.iter().map(|p| [p[0] + 0.5, p[2] + 0.5]).collect();

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create track bed mesh (flat viaduct segment).
fn create_track_bed_mesh(length: f32, width: f32, thickness: f32) -> Mesh {
    let hl = length / 2.0;
    let hw = width / 2.0;
    let ht = thickness / 2.0;

    let positions = vec![
        // Top
        [-hl, ht, -hw],
        [hl, ht, -hw],
        [hl, ht, hw],
        [-hl, ht, hw],
        // Bottom
        [-hl, -ht, -hw],
        [hl, -ht, -hw],
        [hl, -ht, hw],
        [-hl, -ht, hw],
    ];

    let indices = vec![
        // Top
        0, 1, 2, 0, 2, 3, // Bottom
        4, 6, 5, 4, 7, 6, // Front
        0, 5, 1, 0, 4, 5, // Back
        2, 7, 3, 2, 6, 7, // Left
        0, 3, 7, 0, 7, 4, // Right
        1, 5, 6, 1, 6, 2,
    ];

    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];

    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create platform mesh.
fn create_platform_mesh(length: f32, width: f32, height: f32) -> Mesh {
    Cuboid::new(length, height, width).into()
}

//! Bus stops with shelters along major roads.
//!
//! Spawns bus stop shelters at intervals along major roads, with benches,
//! signs, and transparent shelter roofs.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;

pub struct BusStopsPlugin;

impl Plugin for BusStopsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BusStopConfig>()
            .init_resource::<BusStopsSpawned>()
            .add_systems(Update, spawn_bus_stops.run_if(should_spawn_bus_stops));
    }
}

/// Marker resource to prevent bus stops from spawning multiple times.
#[derive(Resource, Default)]
pub struct BusStopsSpawned(pub bool);

fn should_spawn_bus_stops(
    road_mesh: Query<&RoadMeshGenerated>,
    spawned: Res<BusStopsSpawned>,
) -> bool {
    !road_mesh.is_empty() && !spawned.0
}

/// Bus stop marker component.
#[derive(Component)]
pub struct BusStop {
    /// Unique ID for this bus stop.
    pub id: u32,
    /// Position along the road edge (0.0 to 1.0).
    pub road_progress: f32,
    /// Which side of the road (true = right, false = left).
    pub right_side: bool,
}

/// Configuration for bus stop spawning.
#[derive(Resource)]
pub struct BusStopConfig {
    pub seed: u64,
    /// Minimum distance between bus stops (meters).
    pub min_spacing: f32,
    /// Distance from road center to bus stop.
    pub road_offset: f32,
    /// Shelter dimensions.
    pub shelter_width: f32,
    pub shelter_depth: f32,
    pub shelter_height: f32,
    /// Probability of spawning a bus stop on eligible road segments.
    pub spawn_probability: f32,
}

impl Default for BusStopConfig {
    fn default() -> Self {
        Self {
            seed: 44444,
            min_spacing: 80.0,
            road_offset: 5.5,
            shelter_width: 3.0,
            shelter_depth: 1.5,
            shelter_height: 2.5,
            spawn_probability: 0.4,
        }
    }
}

fn spawn_bus_stops(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<BusStopConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BusStopsSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut bus_stop_count = 0;
    let mut bus_stop_id = 0u32;

    // Materials
    let shelter_frame_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let shelter_roof_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.8, 0.9, 0.6),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });

    let bench_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.25, 0.15),
        perceptual_roughness: 0.8,
        ..default()
    });

    let sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.7),
        emissive: LinearRgba::new(0.05, 0.1, 0.2, 1.0),
        ..default()
    });

    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.42),
        metallic: 0.6,
        ..default()
    });

    // Meshes
    let pole_mesh = meshes.add(Cylinder::new(0.05, 2.5));
    let sign_mesh = meshes.add(Cuboid::new(0.6, 0.4, 0.05));
    let roof_mesh = meshes.add(Cuboid::new(config.shelter_width, 0.1, config.shelter_depth));
    let post_mesh = meshes.add(Cylinder::new(0.04, config.shelter_height));
    let bench_seat_mesh = meshes.add(Cuboid::new(config.shelter_width * 0.8, 0.08, 0.4));
    let bench_back_mesh = meshes.add(Cuboid::new(config.shelter_width * 0.8, 0.5, 0.05));

    // Track positions to enforce spacing
    let mut placed_positions: Vec<Vec2> = Vec::new();

    // Iterate over all road edges
    for edge in road_graph.edges() {
        // Only place bus stops on major roads and highways
        if !matches!(edge.road_type, RoadType::Major | RoadType::Highway) {
            continue;
        }

        // Skip short segments
        if edge.length < config.min_spacing {
            continue;
        }

        // Random chance to skip
        if rng.gen::<f32>() > config.spawn_probability {
            continue;
        }

        // Place bus stop at middle of segment
        let progress = 0.5;
        let (pos, dir) = interpolate_edge_position(&edge.points, progress);

        // Check spacing from existing bus stops
        let too_close = placed_positions.iter().any(|p| p.distance(pos) < config.min_spacing);
        if too_close {
            continue;
        }

        // Place on right side of road (in direction of travel)
        let perp = Vec2::new(-dir.y, dir.x);
        let stop_pos = pos + perp * config.road_offset;

        // Calculate rotation to face the road
        let angle = dir.y.atan2(dir.x);
        let rotation = Quat::from_rotation_y(-angle + PI / 2.0);

        // Spawn bus stop parent entity
        let stop_entity = commands.spawn((
            Transform::from_xyz(stop_pos.x, 0.0, stop_pos.y).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            BusStop {
                id: bus_stop_id,
                road_progress: progress,
                right_side: true,
            },
        )).with_children(|parent| {
            // Sign pole
            parent.spawn((
                Mesh3d(pole_mesh.clone()),
                MeshMaterial3d(pole_material.clone()),
                Transform::from_xyz(-config.shelter_width / 2.0 - 0.5, 1.25, 0.0),
            ));

            // Bus stop sign
            parent.spawn((
                Mesh3d(sign_mesh.clone()),
                MeshMaterial3d(sign_material.clone()),
                Transform::from_xyz(-config.shelter_width / 2.0 - 0.5, 2.3, 0.0),
            ));

            // Shelter posts (4 corners)
            for (x, z) in [
                (-config.shelter_width / 2.0 + 0.1, -config.shelter_depth / 2.0 + 0.1),
                (config.shelter_width / 2.0 - 0.1, -config.shelter_depth / 2.0 + 0.1),
                (-config.shelter_width / 2.0 + 0.1, config.shelter_depth / 2.0 - 0.1),
                (config.shelter_width / 2.0 - 0.1, config.shelter_depth / 2.0 - 0.1),
            ] {
                parent.spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(shelter_frame_material.clone()),
                    Transform::from_xyz(x, config.shelter_height / 2.0, z),
                ));
            }

            // Shelter roof
            parent.spawn((
                Mesh3d(roof_mesh.clone()),
                MeshMaterial3d(shelter_roof_material.clone()),
                Transform::from_xyz(0.0, config.shelter_height, 0.0),
            ));

            // Bench seat
            parent.spawn((
                Mesh3d(bench_seat_mesh.clone()),
                MeshMaterial3d(bench_material.clone()),
                Transform::from_xyz(0.0, 0.45, config.shelter_depth / 2.0 - 0.3),
            ));

            // Bench back
            parent.spawn((
                Mesh3d(bench_back_mesh.clone()),
                MeshMaterial3d(bench_material.clone()),
                Transform::from_xyz(0.0, 0.7, config.shelter_depth / 2.0 - 0.05),
            ));
        }).id();

        placed_positions.push(stop_pos);
        bus_stop_id += 1;
        bus_stop_count += 1;
    }

    info!("Spawned {} bus stops with shelters", bus_stop_count);
}

/// Interpolate position and direction along edge waypoints.
fn interpolate_edge_position(points: &[Vec2], progress: f32) -> (Vec2, Vec2) {
    if points.is_empty() {
        return (Vec2::ZERO, Vec2::X);
    }
    if points.len() == 1 {
        return (points[0], Vec2::X);
    }

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

    let last = *points.last().unwrap();
    let dir = if points.len() >= 2 {
        (points[points.len() - 1] - points[points.len() - 2]).normalize_or_zero()
    } else {
        Vec2::X
    };
    (last, dir)
}

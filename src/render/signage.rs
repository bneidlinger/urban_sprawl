//! Street signage: street name signs, traffic signs, business signs.
//!
//! Spawns various signage throughout the city at intersections and along roads.

use bevy::prelude::*;
use petgraph::graph::NodeIndex;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::roads::{RoadGraph, RoadNodeType, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;

pub struct SignagePlugin;

impl Plugin for SignagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SignageConfig>()
            .init_resource::<SignageSpawned>()
            .add_systems(Update, spawn_signage.run_if(should_spawn_signage));
    }
}

#[derive(Resource, Default)]
pub struct SignageSpawned(pub bool);

fn should_spawn_signage(
    road_mesh: Query<&RoadMeshGenerated>,
    spawned: Res<SignageSpawned>,
) -> bool {
    !road_mesh.is_empty() && !spawned.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SignType {
    StreetName,
    StopSign,
    SpeedLimit,
    OneWay,
    Yield,
    NoParking,
}

/// Marker component for signs.
#[derive(Component)]
pub struct StreetSign {
    pub sign_type: SignType,
}

#[derive(Resource)]
pub struct SignageConfig {
    pub seed: u64,
    pub sign_height: f32,
    pub pole_radius: f32,
    pub street_sign_prob: f32,
    pub stop_sign_prob: f32,
    pub speed_sign_prob: f32,
}

impl Default for SignageConfig {
    fn default() -> Self {
        Self {
            seed: 33333,
            sign_height: 2.5,
            pole_radius: 0.04,
            street_sign_prob: 0.7,
            stop_sign_prob: 0.5,
            speed_sign_prob: 0.3,
        }
    }
}

fn spawn_signage(
    mut commands: Commands,
    config: Res<SignageConfig>,
    road_graph: Res<RoadGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<SignageSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut sign_count = 0;

    // Materials
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.42),
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..default()
    });

    let green_sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.4, 0.15),
        perceptual_roughness: 0.5,
        ..default()
    });

    let red_sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.15, 0.1),
        perceptual_roughness: 0.5,
        ..default()
    });

    let white_sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.6,
        ..default()
    });

    let yellow_sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.85, 0.2),
        perceptual_roughness: 0.5,
        ..default()
    });

    let blue_sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.3, 0.6),
        perceptual_roughness: 0.5,
        ..default()
    });

    // Meshes
    let pole_mesh = meshes.add(Cylinder::new(config.pole_radius, config.sign_height));
    let street_sign_mesh = meshes.add(Cuboid::new(1.2, 0.25, 0.02));
    let stop_sign_mesh = meshes.add(create_octagon_mesh(0.35));
    let speed_sign_mesh = meshes.add(Cuboid::new(0.5, 0.6, 0.02));
    let yield_sign_mesh = meshes.add(create_triangle_mesh(0.4));
    let one_way_mesh = meshes.add(Cuboid::new(0.8, 0.25, 0.02));

    // Iterate over intersection nodes
    for (node_idx, node) in road_graph.nodes() {
        let neighbor_count = road_graph.neighbors(node_idx).count();

        // Only place signs at intersections (2+ neighbors)
        if neighbor_count < 2 {
            continue;
        }

        let node_pos = node.position;
        let is_major_intersection = neighbor_count >= 3
            || matches!(node.node_type, RoadNodeType::Intersection);

        // Get outgoing directions for sign placement
        let mut directions: Vec<Vec2> = Vec::new();
        for edge in road_graph.edges_of_node(node_idx) {
            if let Some(edge_data) = road_graph.edge_by_index(edge) {
                if edge_data.points.len() >= 2 {
                    let dir = if edge_data.points[0].distance(node_pos) < 1.0 {
                        (edge_data.points[1] - edge_data.points[0]).normalize_or_zero()
                    } else {
                        (edge_data.points[edge_data.points.len() - 2]
                            - edge_data.points[edge_data.points.len() - 1])
                        .normalize_or_zero()
                    };
                    directions.push(dir);
                }
            }
        }

        if directions.is_empty() {
            continue;
        }

        // Place street name signs at major intersections
        if is_major_intersection && rng.gen::<f32>() < config.street_sign_prob {
            let dir = directions[rng.gen_range(0..directions.len())];
            let perp = Vec2::new(-dir.y, dir.x);
            let sign_pos = node_pos + perp * 6.0 + dir * 2.0;
            let angle = dir.y.atan2(dir.x);

            commands
                .spawn((
                    Transform::from_xyz(sign_pos.x, 0.0, sign_pos.y)
                        .with_rotation(Quat::from_rotation_y(-angle)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    StreetSign { sign_type: SignType::StreetName },
                ))
                .with_children(|parent| {
                    // Pole
                    parent.spawn((
                        Mesh3d(pole_mesh.clone()),
                        MeshMaterial3d(pole_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height / 2.0, 0.0),
                    ));

                    // Street name sign (horizontal)
                    parent.spawn((
                        Mesh3d(street_sign_mesh.clone()),
                        MeshMaterial3d(green_sign_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height - 0.1, 0.0),
                    ));

                    // Cross street sign (perpendicular)
                    if directions.len() >= 2 {
                        parent.spawn((
                            Mesh3d(street_sign_mesh.clone()),
                            MeshMaterial3d(green_sign_material.clone()),
                            Transform::from_xyz(0.0, config.sign_height - 0.4, 0.0)
                                .with_rotation(Quat::from_rotation_y(PI / 2.0)),
                        ));
                    }
                });

            sign_count += 1;
        }

        // Place stop signs at some intersections
        if is_major_intersection && rng.gen::<f32>() < config.stop_sign_prob {
            for (i, dir) in directions.iter().enumerate() {
                if rng.gen::<f32>() > 0.5 {
                    continue;
                }

                let perp = Vec2::new(-dir.y, dir.x);
                let offset_side = if i % 2 == 0 { 1.0 } else { -1.0 };
                let sign_pos = node_pos + perp * 5.0 * offset_side - *dir * 3.0;
                let angle = dir.y.atan2(dir.x);

                commands
                    .spawn((
                        Transform::from_xyz(sign_pos.x, 0.0, sign_pos.y)
                            .with_rotation(Quat::from_rotation_y(-angle + PI)),
                        GlobalTransform::default(),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                        StreetSign { sign_type: SignType::StopSign },
                    ))
                    .with_children(|parent| {
                        // Pole
                        parent.spawn((
                            Mesh3d(pole_mesh.clone()),
                            MeshMaterial3d(pole_material.clone()),
                            Transform::from_xyz(0.0, config.sign_height / 2.0, 0.0),
                        ));

                        // Stop sign
                        parent.spawn((
                            Mesh3d(stop_sign_mesh.clone()),
                            MeshMaterial3d(red_sign_material.clone()),
                            Transform::from_xyz(0.0, config.sign_height - 0.2, 0.0),
                        ));
                    });

                sign_count += 1;
                break; // One stop sign per intersection corner
            }
        }

        // Place speed limit signs on major roads
        if rng.gen::<f32>() < config.speed_sign_prob {
            let dir = directions[rng.gen_range(0..directions.len())];
            let perp = Vec2::new(-dir.y, dir.x);
            let sign_pos = node_pos + perp * 6.5 + dir * 8.0;
            let angle = dir.y.atan2(dir.x);

            commands
                .spawn((
                    Transform::from_xyz(sign_pos.x, 0.0, sign_pos.y)
                        .with_rotation(Quat::from_rotation_y(-angle + PI)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    StreetSign { sign_type: SignType::SpeedLimit },
                ))
                .with_children(|parent| {
                    // Pole
                    parent.spawn((
                        Mesh3d(pole_mesh.clone()),
                        MeshMaterial3d(pole_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height / 2.0, 0.0),
                    ));

                    // Speed limit sign (white rectangle)
                    parent.spawn((
                        Mesh3d(speed_sign_mesh.clone()),
                        MeshMaterial3d(white_sign_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height - 0.2, 0.0),
                    ));
                });

            sign_count += 1;
        }

        // Occasionally add one-way signs
        if neighbor_count == 2 && rng.gen::<f32>() < 0.2 {
            let dir = directions[0];
            let perp = Vec2::new(-dir.y, dir.x);
            let sign_pos = node_pos + perp * 5.5;
            let angle = dir.y.atan2(dir.x);

            commands
                .spawn((
                    Transform::from_xyz(sign_pos.x, 0.0, sign_pos.y)
                        .with_rotation(Quat::from_rotation_y(-angle)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    StreetSign { sign_type: SignType::OneWay },
                ))
                .with_children(|parent| {
                    // Pole
                    parent.spawn((
                        Mesh3d(pole_mesh.clone()),
                        MeshMaterial3d(pole_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height / 2.0 - 0.3, 0.0),
                    ));

                    // One-way sign
                    parent.spawn((
                        Mesh3d(one_way_mesh.clone()),
                        MeshMaterial3d(blue_sign_material.clone()),
                        Transform::from_xyz(0.0, config.sign_height - 0.5, 0.0),
                    ));
                });

            sign_count += 1;
        }
    }

    info!("Spawned {} street signs", sign_count);
}

/// Create an octagonal mesh for stop signs.
fn create_octagon_mesh(radius: f32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};

    let mut positions = vec![[0.0, 0.0, 0.0]]; // Center
    let mut normals = vec![[0.0, 0.0, 1.0]];
    let mut uvs = vec![[0.5, 0.5]];

    // 8 vertices around the octagon
    for i in 0..8 {
        let angle = (i as f32 / 8.0) * PI * 2.0 + PI / 8.0;
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        positions.push([x, y, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.5 + x / radius * 0.5, 0.5 + y / radius * 0.5]);
    }

    // Triangles from center to edges
    let mut indices = Vec::new();
    for i in 0..8 {
        indices.push(0);
        indices.push(i + 1);
        indices.push(if i == 7 { 1 } else { i + 2 });
    }

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a triangular mesh for yield signs.
fn create_triangle_mesh(size: f32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};

    let h = size * 0.866; // Height of equilateral triangle

    let positions = vec![
        [0.0, h * 0.67, 0.0],      // Top
        [-size / 2.0, -h * 0.33, 0.0], // Bottom left
        [size / 2.0, -h * 0.33, 0.0],  // Bottom right
    ];

    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];

    let uvs = vec![
        [0.5, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
    ];

    let indices = vec![0, 2, 1]; // CCW winding

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

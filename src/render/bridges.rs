//! Bridge rendering for roads that cross water.
//!
//! Creates bridge meshes with elevated deck, railings, and support pillars.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::procgen::river::River;
use crate::procgen::road_generator::RoadsGenerated;
use crate::procgen::roads::{RoadGraph, RoadType};

pub struct BridgesPlugin;

impl Plugin for BridgesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BridgeConfig>()
            .init_resource::<BridgesSpawned>()
            .add_systems(Update, spawn_bridges.run_if(should_spawn_bridges));
    }
}

fn should_spawn_bridges(generated: Res<RoadsGenerated>, spawned: Res<BridgesSpawned>) -> bool {
    generated.0 && !spawned.0
}

/// Marker resource indicating bridges have been spawned.
#[derive(Resource, Default)]
pub struct BridgesSpawned(pub bool);

/// Configuration for bridge rendering.
#[derive(Resource)]
pub struct BridgeConfig {
    /// Height of bridge deck above water.
    pub deck_height: f32,
    /// Thickness of the bridge deck.
    pub deck_thickness: f32,
    /// Height of railings.
    pub railing_height: f32,
    /// Width of railing posts.
    pub railing_width: f32,
    /// Number of support pillars.
    pub pillar_count: usize,
    /// Width of support pillars.
    pub pillar_width: f32,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            deck_height: 1.5,
            deck_thickness: 0.4,
            railing_height: 0.8,
            railing_width: 0.15,
            pillar_count: 2,
            pillar_width: 1.0,
        }
    }
}

/// Marker component for bridge entities.
#[derive(Component)]
pub struct Bridge {
    pub road_type: RoadType,
}

/// Marker component for bridge railing.
#[derive(Component)]
pub struct BridgeRailing;

/// Marker component for bridge pillar.
#[derive(Component)]
pub struct BridgePillar;

/// Spawn bridge meshes for all water crossings.
fn spawn_bridges(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    river: Res<River>,
    config: Res<BridgeConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BridgesSpawned>,
) {
    info!("Spawning bridges...");

    // Bridge deck material (concrete/asphalt)
    let deck_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.45),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Railing material (metal)
    let railing_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    // Pillar material (concrete)
    let pillar_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.48, 0.45),
        perceptual_roughness: 0.9,
        ..default()
    });

    let mut bridge_count = 0;

    // Find all edges that cross water
    for edge in road_graph.edges() {
        if !edge.crosses_water {
            continue;
        }

        let (entry, exit) = match (edge.water_entry, edge.water_exit) {
            (Some(e), Some(x)) => (e, x),
            _ => continue,
        };

        let road_width = match edge.road_type {
            RoadType::Highway => 12.0,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => 3.0,
        };

        // Calculate bridge geometry
        let direction = (exit - entry).normalize_or_zero();
        let perpendicular = Vec2::new(-direction.y, direction.x);
        let bridge_length = entry.distance(exit);
        let center = (entry + exit) / 2.0;

        // Bridge deck height (above water)
        let deck_y = river.water_level + config.deck_height;

        // Create bridge deck mesh
        let deck_mesh = create_bridge_deck(
            entry,
            exit,
            road_width,
            config.deck_thickness,
            deck_y,
        );

        commands.spawn((
            Mesh3d(meshes.add(deck_mesh)),
            MeshMaterial3d(deck_material.clone()),
            Transform::IDENTITY,
            Bridge {
                road_type: edge.road_type,
            },
        ));

        // Create railings on both sides
        let railing_offset = road_width / 2.0 + config.railing_width / 2.0;

        for side in [-1.0, 1.0] {
            let railing_mesh = create_railing(
                entry + perpendicular * railing_offset * side,
                exit + perpendicular * railing_offset * side,
                config.railing_width,
                config.railing_height,
                deck_y,
            );

            commands.spawn((
                Mesh3d(meshes.add(railing_mesh)),
                MeshMaterial3d(railing_material.clone()),
                Transform::IDENTITY,
                BridgeRailing,
            ));
        }

        // Create support pillars
        if config.pillar_count > 0 && bridge_length > 10.0 {
            let pillar_depth = river.water_level - (-3.0); // Extend below water

            for i in 0..config.pillar_count {
                let t = (i as f32 + 1.0) / (config.pillar_count as f32 + 1.0);
                let pillar_pos = entry.lerp(exit, t);

                let pillar_mesh = create_pillar(
                    pillar_pos,
                    config.pillar_width,
                    pillar_depth + config.deck_height,
                    deck_y - config.deck_thickness,
                );

                commands.spawn((
                    Mesh3d(meshes.add(pillar_mesh)),
                    MeshMaterial3d(pillar_material.clone()),
                    Transform::IDENTITY,
                    BridgePillar,
                ));
            }
        }

        bridge_count += 1;
    }

    spawned.0 = true;
    info!("Spawned {} bridges", bridge_count);
}

/// Create a bridge deck mesh (flat box along the crossing).
fn create_bridge_deck(
    start: Vec2,
    end: Vec2,
    width: f32,
    thickness: f32,
    top_y: f32,
) -> Mesh {
    let direction = (end - start).normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let half_width = width / 2.0;

    // Four corners of the deck (top surface)
    let corners = [
        start + perpendicular * half_width,  // top-left start
        start - perpendicular * half_width,  // top-right start
        end + perpendicular * half_width,    // top-left end
        end - perpendicular * half_width,    // top-right end
    ];

    let bottom_y = top_y - thickness;

    // 8 vertices (4 top, 4 bottom)
    let vertices: Vec<[f32; 3]> = vec![
        // Top face
        [corners[0].x, top_y, corners[0].y], // 0
        [corners[1].x, top_y, corners[1].y], // 1
        [corners[2].x, top_y, corners[2].y], // 2
        [corners[3].x, top_y, corners[3].y], // 3
        // Bottom face
        [corners[0].x, bottom_y, corners[0].y], // 4
        [corners[1].x, bottom_y, corners[1].y], // 5
        [corners[2].x, bottom_y, corners[2].y], // 6
        [corners[3].x, bottom_y, corners[3].y], // 7
    ];

    // Normals (simplified - all faces have their respective normals)
    let normals: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
    ];

    // Indices for all 6 faces (CCW winding)
    let indices: Vec<u32> = vec![
        // Top face
        0, 2, 1, 1, 2, 3,
        // Bottom face
        4, 5, 6, 5, 7, 6,
        // Front face (start)
        0, 1, 4, 1, 5, 4,
        // Back face (end)
        2, 6, 3, 3, 6, 7,
        // Left face
        0, 4, 2, 2, 4, 6,
        // Right face
        1, 3, 5, 3, 7, 5,
    ];

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a railing mesh along the bridge edge.
fn create_railing(
    start: Vec2,
    end: Vec2,
    width: f32,
    height: f32,
    deck_y: f32,
) -> Mesh {
    let direction = (end - start).normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let half_width = width / 2.0;

    let corners = [
        start + perpendicular * half_width,
        start - perpendicular * half_width,
        end + perpendicular * half_width,
        end - perpendicular * half_width,
    ];

    let bottom_y = deck_y;
    let top_y = deck_y + height;

    let vertices: Vec<[f32; 3]> = vec![
        [corners[0].x, top_y, corners[0].y],
        [corners[1].x, top_y, corners[1].y],
        [corners[2].x, top_y, corners[2].y],
        [corners[3].x, top_y, corners[3].y],
        [corners[0].x, bottom_y, corners[0].y],
        [corners[1].x, bottom_y, corners[1].y],
        [corners[2].x, bottom_y, corners[2].y],
        [corners[3].x, bottom_y, corners[3].y],
    ];

    let normals: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
    ];

    let indices: Vec<u32> = vec![
        0, 2, 1, 1, 2, 3,
        4, 5, 6, 5, 7, 6,
        0, 1, 4, 1, 5, 4,
        2, 6, 3, 3, 6, 7,
        0, 4, 2, 2, 4, 6,
        1, 3, 5, 3, 7, 5,
    ];

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a support pillar mesh.
fn create_pillar(
    position: Vec2,
    width: f32,
    height: f32,
    top_y: f32,
) -> Mesh {
    let half_width = width / 2.0;
    let bottom_y = top_y - height;

    // Simple box pillar
    let vertices: Vec<[f32; 3]> = vec![
        // Top face
        [position.x - half_width, top_y, position.y - half_width],
        [position.x + half_width, top_y, position.y - half_width],
        [position.x - half_width, top_y, position.y + half_width],
        [position.x + half_width, top_y, position.y + half_width],
        // Bottom face
        [position.x - half_width, bottom_y, position.y - half_width],
        [position.x + half_width, bottom_y, position.y - half_width],
        [position.x - half_width, bottom_y, position.y + half_width],
        [position.x + half_width, bottom_y, position.y + half_width],
    ];

    let normals: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0],
    ];

    let indices: Vec<u32> = vec![
        0, 2, 1, 1, 2, 3,
        4, 5, 6, 5, 7, 6,
        0, 1, 4, 1, 5, 4,
        2, 6, 3, 3, 6, 7,
        0, 4, 2, 2, 4, 6,
        1, 3, 5, 3, 7, 5,
    ];

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

//! Balconies for residential buildings.
//!
//! Spawns balconies on the facades of residential buildings at regular intervals.
//! Balconies are small protruding platforms with railings.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct BalconiesPlugin;

impl Plugin for BalconiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BalconyConfig>()
            .init_resource::<BalconiesSpawned>()
            .add_systems(Update, spawn_balconies.run_if(should_spawn_balconies));
    }
}

/// Marker resource to prevent balcony system from running multiple times.
#[derive(Resource, Default)]
pub struct BalconiesSpawned(pub bool);

fn should_spawn_balconies(
    spawned: Res<BuildingsSpawned>,
    balconies_spawned: Res<BalconiesSpawned>,
) -> bool {
    spawned.0 && !balconies_spawned.0
}

/// Marker component for balcony entities.
#[derive(Component)]
pub struct Balcony;

/// Configuration for balcony spawning.
#[derive(Resource)]
pub struct BalconyConfig {
    pub seed: u64,
    /// Probability of a residential building having balconies.
    pub building_probability: f32,
    /// Vertical spacing between balcony floors.
    pub floor_height: f32,
    /// Balcony depth (how far it protrudes).
    pub balcony_depth: f32,
    /// Balcony width.
    pub balcony_width: f32,
    /// Railing height.
    pub railing_height: f32,
    /// Minimum building height to have balconies.
    pub min_building_height: f32,
}

impl Default for BalconyConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            building_probability: 0.6,
            floor_height: 3.0,
            balcony_depth: 1.2,
            balcony_width: 2.5,
            railing_height: 1.0,
            min_building_height: 6.0, // At least 2 floors
        }
    }
}

fn spawn_balconies(
    mut commands: Commands,
    config: Res<BalconyConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut balconies_spawned: ResMut<BalconiesSpawned>,
) {
    balconies_spawned.0 = true;

    info!("Spawning balconies on residential buildings...");

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Create balcony meshes
    let floor_mesh = meshes.add(create_balcony_floor_mesh(
        config.balcony_width,
        config.balcony_depth,
    ));
    let railing_mesh = meshes.add(create_balcony_railing_mesh(
        config.balcony_width,
        config.balcony_depth,
        config.railing_height,
    ));

    // Materials
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.5, 0.48),
        perceptual_roughness: 0.8,
        ..default()
    });

    let railing_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let mut balcony_count = 0;
    let mut eligible_buildings = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Only residential buildings get balconies
        if building.building_type != BuildingArchetype::Residential {
            continue;
        }

        // Random chance for this building to have balconies
        if rng.gen::<f32>() > config.building_probability {
            continue;
        }

        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        // Get world-space dimensions
        let scale = transform.scale;
        let building_height = aabb.half_extents.y * 2.0 * scale.y;
        let building_width = aabb.half_extents.x * 2.0 * scale.x;
        let building_depth = aabb.half_extents.z * 2.0 * scale.z;

        // Skip buildings that are too short
        if building_height < config.min_building_height {
            continue;
        }

        eligible_buildings += 1;

        let pos = transform.translation;
        let base_y = pos.y - building_height / 2.0;

        // Determine which faces get balconies (usually front and back, not sides)
        let faces = determine_balcony_faces(&mut rng, building_width, building_depth);

        // Calculate number of floors
        let num_floors = ((building_height - config.floor_height) / config.floor_height) as i32;

        for floor in 1..=num_floors {
            let floor_y = base_y + floor as f32 * config.floor_height;

            // Skip some floors randomly for variety
            if rng.gen::<f32>() < 0.2 {
                continue;
            }

            for &(face_dir, face_width) in &faces {
                // Calculate balconies per row
                let balconies_per_row =
                    ((face_width - 1.0) / (config.balcony_width + 1.5)).floor() as i32;

                if balconies_per_row < 1 {
                    continue;
                }

                let spacing =
                    (face_width - balconies_per_row as f32 * config.balcony_width)
                        / (balconies_per_row + 1) as f32;

                for i in 0..balconies_per_row {
                    // Random chance to skip individual balconies for variation
                    if rng.gen::<f32>() < 0.15 {
                        continue;
                    }

                    let offset_along_face = -face_width / 2.0
                        + spacing
                        + config.balcony_width / 2.0
                        + i as f32 * (config.balcony_width + spacing);

                    let (balcony_x, balcony_z, rotation) =
                        calculate_balcony_position(pos, face_dir, offset_along_face, &config);

                    // Floor platform
                    commands.spawn((
                        Mesh3d(floor_mesh.clone()),
                        MeshMaterial3d(floor_material.clone()),
                        Transform::from_xyz(balcony_x, floor_y, balcony_z)
                            .with_rotation(rotation),
                        Balcony,
                    ));

                    // Railing
                    commands.spawn((
                        Mesh3d(railing_mesh.clone()),
                        MeshMaterial3d(railing_material.clone()),
                        Transform::from_xyz(balcony_x, floor_y + 0.1, balcony_z)
                            .with_rotation(rotation),
                        Balcony,
                    ));

                    balcony_count += 1;
                }
            }
        }
    }

    info!(
        "Spawned {} balconies on {} residential buildings",
        balcony_count, eligible_buildings
    );
}

/// Determine which faces of the building get balconies.
/// Returns a list of (direction_vector, face_width).
fn determine_balcony_faces(
    rng: &mut StdRng,
    building_width: f32,
    building_depth: f32,
) -> Vec<(Vec3, f32)> {
    let mut faces = Vec::new();

    // Typically add balconies to 1-2 faces
    let num_faces = if rng.gen::<bool>() { 2 } else { 1 };

    // Prefer wider faces
    let face_options = [
        (Vec3::X, building_depth),  // Right face
        (Vec3::NEG_X, building_depth), // Left face
        (Vec3::Z, building_width),  // Front face
        (Vec3::NEG_Z, building_width), // Back face
    ];

    // Shuffle and pick
    let mut indices: Vec<usize> = (0..4).collect();
    for i in (1..4).rev() {
        let j = rng.gen_range(0..=i);
        indices.swap(i, j);
    }

    for &idx in indices.iter().take(num_faces) {
        faces.push(face_options[idx]);
    }

    faces
}

/// Calculate the world position and rotation for a balcony.
fn calculate_balcony_position(
    building_pos: Vec3,
    face_direction: Vec3,
    offset_along_face: f32,
    config: &BalconyConfig,
) -> (f32, f32, Quat) {
    let half_depth = config.balcony_depth / 2.0;

    // Calculate position based on face direction
    let (x, z, rotation) = if face_direction.x > 0.5 {
        // Right face (+X)
        (
            building_pos.x + half_depth,
            building_pos.z + offset_along_face,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        )
    } else if face_direction.x < -0.5 {
        // Left face (-X)
        (
            building_pos.x - half_depth,
            building_pos.z + offset_along_face,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        )
    } else if face_direction.z > 0.5 {
        // Front face (+Z)
        (
            building_pos.x + offset_along_face,
            building_pos.z + half_depth,
            Quat::IDENTITY,
        )
    } else {
        // Back face (-Z)
        (
            building_pos.x + offset_along_face,
            building_pos.z - half_depth,
            Quat::from_rotation_y(std::f32::consts::PI),
        )
    };

    (x, z, rotation)
}

/// Create a simple floor mesh for the balcony platform.
fn create_balcony_floor_mesh(width: f32, depth: f32) -> Mesh {
    let hw = width / 2.0;
    let hd = depth / 2.0;
    let thickness = 0.1;

    let vertices = vec![
        // Top face
        [-hw, thickness, -hd],
        [hw, thickness, -hd],
        [hw, thickness, hd],
        [-hw, thickness, hd],
        // Bottom face
        [-hw, 0.0, -hd],
        [hw, 0.0, -hd],
        [hw, 0.0, hd],
        [-hw, 0.0, hd],
    ];

    let indices = vec![
        // Top
        0, 2, 1, 0, 3, 2, // Bottom
        4, 5, 6, 4, 6, 7, // Sides
        0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
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

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create a railing mesh (3 sides of the balcony).
fn create_balcony_railing_mesh(width: f32, depth: f32, height: f32) -> Mesh {
    let hw = width / 2.0;
    let bar_radius = 0.03;
    let spacing = 0.15;

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Helper to add a vertical bar (simplified box)
    let add_bar = |vertices: &mut Vec<[f32; 3]>,
                   normals: &mut Vec<[f32; 3]>,
                   uvs: &mut Vec<[f32; 2]>,
                   indices: &mut Vec<u32>,
                   x: f32,
                   z: f32,
                   h: f32| {
        let base_idx = vertices.len() as u32;
        let r = bar_radius;

        // Simple box for bar
        let verts = [
            [x - r, 0.0, z - r],
            [x + r, 0.0, z - r],
            [x + r, 0.0, z + r],
            [x - r, 0.0, z + r],
            [x - r, h, z - r],
            [x + r, h, z - r],
            [x + r, h, z + r],
            [x - r, h, z + r],
        ];

        for v in verts {
            vertices.push(v);
            normals.push([0.0, 1.0, 0.0]); // Simplified normal
            uvs.push([0.0, 0.0]);
        }

        // Side faces
        let faces = [
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
            [4, 5, 6, 7], // Top
        ];

        for face in faces {
            indices.push(base_idx + face[0]);
            indices.push(base_idx + face[1]);
            indices.push(base_idx + face[2]);
            indices.push(base_idx + face[0]);
            indices.push(base_idx + face[2]);
            indices.push(base_idx + face[3]);
        }
    };

    // Helper to add a horizontal rail
    let add_rail = |vertices: &mut Vec<[f32; 3]>,
                    normals: &mut Vec<[f32; 3]>,
                    uvs: &mut Vec<[f32; 2]>,
                    indices: &mut Vec<u32>,
                    x1: f32,
                    z1: f32,
                    x2: f32,
                    z2: f32,
                    y: f32| {
        let base_idx = vertices.len() as u32;
        let r = bar_radius;

        // Calculate direction and perpendicular
        let dx = x2 - x1;
        let dz = z2 - z1;
        let len = (dx * dx + dz * dz).sqrt();
        let px = -dz / len * r;
        let pz = dx / len * r;

        // Box along the rail
        let verts = [
            [x1 + px, y - r, z1 + pz],
            [x1 - px, y - r, z1 - pz],
            [x2 - px, y - r, z2 - pz],
            [x2 + px, y - r, z2 + pz],
            [x1 + px, y + r, z1 + pz],
            [x1 - px, y + r, z1 - pz],
            [x2 - px, y + r, z2 - pz],
            [x2 + px, y + r, z2 + pz],
        ];

        for v in verts {
            vertices.push(v);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([0.0, 0.0]);
        }

        // Side faces
        let faces = [
            [0, 3, 7, 4],
            [1, 0, 4, 5],
            [2, 1, 5, 6],
            [3, 2, 6, 7],
            [4, 7, 6, 5], // Top
        ];

        for face in faces {
            indices.push(base_idx + face[0]);
            indices.push(base_idx + face[1]);
            indices.push(base_idx + face[2]);
            indices.push(base_idx + face[0]);
            indices.push(base_idx + face[2]);
            indices.push(base_idx + face[3]);
        }
    };

    // Front railing (at depth edge)
    let front_z = depth;
    let num_front_bars = ((width - spacing) / spacing) as i32;

    for i in 0..=num_front_bars {
        let x = -hw + spacing / 2.0 + i as f32 * spacing;
        add_bar(&mut vertices, &mut normals, &mut uvs, &mut indices, x, front_z, height);
    }

    // Side railings
    let num_side_bars = ((depth - spacing) / spacing) as i32;

    // Left side
    for i in 0..=num_side_bars {
        let z = i as f32 * spacing;
        add_bar(&mut vertices, &mut normals, &mut uvs, &mut indices, -hw, z, height);
    }

    // Right side
    for i in 0..=num_side_bars {
        let z = i as f32 * spacing;
        add_bar(&mut vertices, &mut normals, &mut uvs, &mut indices, hw, z, height);
    }

    // Top horizontal rails
    add_rail(
        &mut vertices,
        &mut normals,
        &mut uvs,
        &mut indices,
        -hw,
        front_z,
        hw,
        front_z,
        height,
    );
    add_rail(
        &mut vertices,
        &mut normals,
        &mut uvs,
        &mut indices,
        -hw,
        0.0,
        -hw,
        front_z,
        height,
    );
    add_rail(
        &mut vertices,
        &mut normals,
        &mut uvs,
        &mut indices,
        hw,
        0.0,
        hw,
        front_z,
        height,
    );

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

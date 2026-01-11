//! Storefronts with awnings for commercial buildings.
//!
//! Adds colorful awnings at ground level of commercial buildings to create
//! a lively street-level appearance.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct StorefrontsPlugin;

impl Plugin for StorefrontsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StorefrontConfig>()
            .init_resource::<StorefrontsSpawned>()
            .add_systems(Update, spawn_storefronts.run_if(should_spawn_storefronts));
    }
}

/// Marker resource to prevent storefront system from running multiple times.
#[derive(Resource, Default)]
pub struct StorefrontsSpawned(pub bool);

fn should_spawn_storefronts(
    spawned: Res<BuildingsSpawned>,
    storefronts_spawned: Res<StorefrontsSpawned>,
) -> bool {
    spawned.0 && !storefronts_spawned.0
}

/// Marker component for awning entities.
#[derive(Component)]
pub struct Awning;

/// Marker component for storefront window entities.
#[derive(Component)]
pub struct StorefrontWindow;

/// Configuration for storefront spawning.
#[derive(Resource)]
pub struct StorefrontConfig {
    pub seed: u64,
    /// Probability of a commercial building having storefronts.
    pub building_probability: f32,
    /// Height from ground to bottom of awning.
    pub awning_height: f32,
    /// Awning width.
    pub awning_width: f32,
    /// Awning depth (how far it extends from building).
    pub awning_depth: f32,
    /// Awning slope angle (radians).
    pub awning_angle: f32,
}

impl Default for StorefrontConfig {
    fn default() -> Self {
        Self {
            seed: 99999,
            building_probability: 0.7,
            awning_height: 2.8,
            awning_width: 3.5,
            awning_depth: 1.5,
            awning_angle: 0.2, // Slight downward slope
        }
    }
}

/// Awning color palette - bright, varied colors.
const AWNING_COLORS: &[(f32, f32, f32)] = &[
    (0.8, 0.2, 0.2),   // Red
    (0.2, 0.5, 0.8),   // Blue
    (0.2, 0.7, 0.3),   // Green
    (0.9, 0.6, 0.1),   // Orange
    (0.6, 0.2, 0.5),   // Purple
    (0.1, 0.6, 0.6),   // Teal
    (0.85, 0.85, 0.2), // Yellow
    (0.8, 0.4, 0.6),   // Pink
];

fn spawn_storefronts(
    mut commands: Commands,
    config: Res<StorefrontConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut storefronts_spawned: ResMut<StorefrontsSpawned>,
) {
    storefronts_spawned.0 = true;

    info!("Spawning storefronts with awnings on commercial buildings...");

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Create awning mesh
    let awning_mesh = meshes.add(create_awning_mesh(
        config.awning_width,
        config.awning_depth,
        config.awning_angle,
    ));

    // Create storefront window mesh (dark glass panel)
    let window_mesh = meshes.add(Cuboid::new(config.awning_width - 0.2, 2.0, 0.05));
    let window_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.15, 0.2, 0.25, 0.9),
        metallic: 0.1,
        perceptual_roughness: 0.2,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Pre-create awning materials
    let awning_materials: Vec<Handle<StandardMaterial>> = AWNING_COLORS
        .iter()
        .map(|&(r, g, b)| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                perceptual_roughness: 0.7,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();

    // Striped awning materials (alternating pattern)
    let striped_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.7,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let mut awning_count = 0;
    let mut window_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Only commercial buildings get storefronts
        if building.building_type != BuildingArchetype::Commercial {
            continue;
        }

        // Random chance for this building to have storefronts
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
        let building_width = aabb.half_extents.x * 2.0 * scale.x;
        let building_depth = aabb.half_extents.z * 2.0 * scale.z;
        let building_height = aabb.half_extents.y * 2.0 * scale.y;

        // Skip small buildings
        if building_width < 4.0 || building_depth < 4.0 {
            continue;
        }

        let pos = transform.translation;
        let base_y = pos.y - building_height / 2.0;

        // Choose random awning color
        let color_idx = rng.gen_range(0..awning_materials.len());
        let awning_material = if rng.gen::<f32>() < 0.2 {
            // 20% chance of white/striped awning
            striped_material.clone()
        } else {
            awning_materials[color_idx].clone()
        };

        // Determine which faces get storefronts (usually 1-2 faces)
        let faces = determine_storefront_faces(&mut rng, building_width, building_depth);

        for &(face_dir, face_width, face_offset) in &faces {
            // Calculate number of storefronts
            let storefronts_count = ((face_width - 0.5) / (config.awning_width + 0.8)).floor() as i32;

            if storefronts_count < 1 {
                continue;
            }

            let spacing = (face_width - storefronts_count as f32 * config.awning_width)
                / (storefronts_count + 1) as f32;

            for i in 0..storefronts_count {
                let offset_along_face = -face_width / 2.0
                    + spacing
                    + config.awning_width / 2.0
                    + i as f32 * (config.awning_width + spacing);

                let (awning_x, awning_z, rotation) = calculate_storefront_position(
                    pos,
                    face_dir,
                    face_offset,
                    offset_along_face,
                    &config,
                );

                // Awning
                commands.spawn((
                    Mesh3d(awning_mesh.clone()),
                    MeshMaterial3d(awning_material.clone()),
                    Transform::from_xyz(awning_x, base_y + config.awning_height, awning_z)
                        .with_rotation(rotation),
                    Awning,
                ));
                awning_count += 1;

                // Storefront window below awning
                let window_y = base_y + config.awning_height - 1.2;
                let window_offset = if face_dir.x.abs() > 0.5 {
                    Vec3::new(face_dir.x.signum() * 0.05, 0.0, 0.0)
                } else {
                    Vec3::new(0.0, 0.0, face_dir.z.signum() * 0.05)
                };

                commands.spawn((
                    Mesh3d(window_mesh.clone()),
                    MeshMaterial3d(window_material.clone()),
                    Transform::from_xyz(
                        awning_x + window_offset.x,
                        window_y,
                        awning_z + window_offset.z,
                    )
                    .with_rotation(rotation),
                    StorefrontWindow,
                ));
                window_count += 1;
            }
        }
    }

    info!(
        "Spawned {} awnings and {} storefront windows",
        awning_count, window_count
    );
}

/// Determine which faces of the building get storefronts.
/// Returns (direction_vector, face_width, offset_from_center).
fn determine_storefront_faces(
    rng: &mut StdRng,
    building_width: f32,
    building_depth: f32,
) -> Vec<(Vec3, f32, f32)> {
    let mut faces = Vec::new();

    // Typically storefronts on 1-2 faces
    let num_faces = if rng.gen::<f32>() < 0.4 { 2 } else { 1 };

    let face_options = [
        (Vec3::X, building_depth, building_width / 2.0),       // Right face
        (Vec3::NEG_X, building_depth, -building_width / 2.0),  // Left face
        (Vec3::Z, building_width, building_depth / 2.0),       // Front face
        (Vec3::NEG_Z, building_width, -building_depth / 2.0),  // Back face
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

/// Calculate the world position and rotation for a storefront.
fn calculate_storefront_position(
    building_pos: Vec3,
    face_direction: Vec3,
    face_offset: f32,
    offset_along_face: f32,
    config: &StorefrontConfig,
) -> (f32, f32, Quat) {
    let depth_offset = config.awning_depth * 0.5;

    let (x, z, rotation) = if face_direction.x > 0.5 {
        // Right face (+X)
        (
            building_pos.x + face_offset + depth_offset,
            building_pos.z + offset_along_face,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        )
    } else if face_direction.x < -0.5 {
        // Left face (-X)
        (
            building_pos.x + face_offset - depth_offset,
            building_pos.z + offset_along_face,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        )
    } else if face_direction.z > 0.5 {
        // Front face (+Z)
        (
            building_pos.x + offset_along_face,
            building_pos.z + face_offset + depth_offset,
            Quat::IDENTITY,
        )
    } else {
        // Back face (-Z)
        (
            building_pos.x + offset_along_face,
            building_pos.z + face_offset - depth_offset,
            Quat::from_rotation_y(std::f32::consts::PI),
        )
    };

    (x, z, rotation)
}

/// Create an awning mesh (sloped canopy with valance).
fn create_awning_mesh(width: f32, depth: f32, angle: f32) -> Mesh {
    let hw = width / 2.0;
    let front_drop = depth * angle.sin();
    let valance_height = 0.25; // Decorative front edge

    // Main canopy vertices
    let vertices = vec![
        // Top surface
        [-hw, 0.0, 0.0],               // Back left
        [hw, 0.0, 0.0],                // Back right
        [hw, -front_drop, depth],      // Front right
        [-hw, -front_drop, depth],     // Front left
        // Bottom surface
        [-hw, -0.05, 0.0],             // Back left (underside)
        [hw, -0.05, 0.0],              // Back right (underside)
        [hw, -front_drop - 0.05, depth], // Front right (underside)
        [-hw, -front_drop - 0.05, depth], // Front left (underside)
        // Valance (decorative front edge)
        [-hw, -front_drop, depth],                     // Valance top left
        [hw, -front_drop, depth],                      // Valance top right
        [hw, -front_drop - valance_height, depth],     // Valance bottom right
        [-hw, -front_drop - valance_height, depth],    // Valance bottom left
    ];

    let indices = vec![
        // Top surface
        0, 3, 1, 1, 3, 2,
        // Bottom surface
        4, 5, 7, 5, 6, 7,
        // Left side
        0, 4, 3, 3, 4, 7,
        // Right side
        1, 2, 5, 2, 6, 5,
        // Valance front
        8, 9, 10, 8, 10, 11,
    ];

    let normals = vec![
        [0.0, 1.0, 0.0], // Top normals
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], // Bottom normals
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0], // Valance front normals
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
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

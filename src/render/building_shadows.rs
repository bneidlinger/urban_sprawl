//! Drop shadows for buildings using alpha-blended offset planes.
//!
//! Creates simple fake shadows beneath buildings for added visual depth.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, PrimitiveTopology};
use noise::{NoiseFn, Perlin};

use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::instancing::TerrainConfig;

pub struct BuildingShadowsPlugin;

impl Plugin for BuildingShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingShadowConfig>()
            .add_systems(Update, spawn_building_shadows.run_if(should_spawn_shadows));
    }
}

fn should_spawn_shadows(
    spawned: Res<BuildingsSpawned>,
    shadow_query: Query<&BuildingShadow>,
) -> bool {
    spawned.0 && shadow_query.is_empty()
}

/// Marker component for building shadow entities.
#[derive(Component)]
pub struct BuildingShadow;

/// Configuration for building drop shadows.
#[derive(Resource)]
pub struct BuildingShadowConfig {
    /// Shadow offset direction (simulates sun angle).
    pub offset_direction: Vec2,
    /// Shadow offset distance multiplier based on building height.
    pub offset_scale: f32,
    /// Shadow opacity (0.0 = invisible, 1.0 = solid black).
    pub opacity: f32,
    /// How much the shadow extends beyond the building footprint.
    pub size_padding: f32,
    /// Height offset above terrain to prevent z-fighting.
    pub height_offset: f32,
    /// Shadow blur/softness (larger = more spread).
    pub spread: f32,
}

impl Default for BuildingShadowConfig {
    fn default() -> Self {
        Self {
            offset_direction: Vec2::new(0.4, 0.3).normalize(),
            offset_scale: 0.15,
            opacity: 0.4,
            size_padding: 1.0,
            height_offset: 0.05,
            spread: 1.2,
        }
    }
}

fn spawn_building_shadows(
    mut commands: Commands,
    config: Res<BuildingShadowConfig>,
    terrain_config: Res<TerrainConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning building shadows...");

    let terrain = TerrainSampler::new(&terrain_config);

    // Semi-transparent shadow material
    let shadow_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.05, config.opacity),
        alpha_mode: AlphaMode::Blend,
        unlit: true, // Shadows shouldn't receive lighting
        ..default()
    });

    let mut shadow_count = 0;

    for (_building, transform, mesh_handle) in building_query.iter() {
        // Get building dimensions from mesh AABB
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        let building_width = aabb.half_extents.x * 2.0;
        let building_height = aabb.half_extents.y * 2.0;
        let building_depth = aabb.half_extents.z * 2.0;

        // Building base position (transform is at center, so base is at y - height/2)
        let building_pos = transform.translation;
        let _base_y = building_pos.y - building_height / 2.0;

        // Shadow offset based on building height
        let shadow_offset = config.offset_direction * building_height * config.offset_scale;

        // Shadow center position
        let shadow_center = Vec2::new(
            building_pos.x + shadow_offset.x,
            building_pos.z + shadow_offset.y,
        );

        // Shadow size (slightly larger than building footprint)
        let shadow_width = (building_width + config.size_padding) * config.spread;
        let shadow_depth = (building_depth + config.size_padding) * config.spread;

        // Sample terrain height at shadow center
        let terrain_height = terrain.sample(shadow_center.x, shadow_center.y);

        // Create shadow mesh (simple quad)
        let shadow_mesh = create_shadow_quad(shadow_width, shadow_depth);

        commands.spawn((
            Mesh3d(meshes.add(shadow_mesh)),
            MeshMaterial3d(shadow_material.clone()),
            Transform::from_xyz(
                shadow_center.x,
                terrain_height + config.height_offset,
                shadow_center.y,
            ),
            BuildingShadow,
        ));

        shadow_count += 1;
    }

    info!("Spawned {} building shadows", shadow_count);
}

/// Create a simple quad mesh for the shadow.
fn create_shadow_quad(width: f32, depth: f32) -> Mesh {
    let hw = width / 2.0;
    let hd = depth / 2.0;

    let vertices = vec![
        [-hw, 0.0, -hd],
        [hw, 0.0, -hd],
        [hw, 0.0, hd],
        [-hw, 0.0, hd],
    ];

    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];

    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];

    // CCW winding
    let indices = vec![0u32, 2, 1, 0, 3, 2];

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

//! Hardware instancing for rendering 100k+ entities efficiently.
//!
//! Uses Bevy's built-in instancing with custom instance data.

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, VertexBufferLayout, VertexFormat,
            VertexStepMode,
        },
    },
    pbr::{MaterialPipeline, MaterialPipelineKey},
};
use bytemuck::{Pod, Zeroable};
use noise::{NoiseFn, Perlin};

pub struct InstancingPlugin;

impl Plugin for InstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<InstancedMaterial>::default())
            .init_resource::<InstancingConfig>()
            .init_resource::<TerrainConfig>()
            .add_systems(Startup, setup_instanced_cubes);
    }
}

/// Configuration for terrain generation.
#[derive(Resource)]
pub struct TerrainConfig {
    /// Size of the terrain in world units.
    pub size: f32,
    /// Number of subdivisions per axis.
    pub resolution: u32,
    /// Maximum height variation from noise.
    pub height_scale: f32,
    /// Noise frequency (higher = more hills).
    pub noise_scale: f32,
    /// Number of octaves for fractal noise.
    pub octaves: u32,
    /// Random seed for noise.
    pub seed: u32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            size: 600.0,
            resolution: 128,
            height_scale: 8.0,
            noise_scale: 0.008,
            octaves: 4,
            seed: 42,
        }
    }
}

/// Configuration for instancing.
#[derive(Resource)]
pub struct InstancingConfig {
    pub instance_count: usize,
    pub grid_size: usize,
}

impl Default for InstancingConfig {
    fn default() -> Self {
        Self {
            instance_count: 0, // Disabled for road visualization
            grid_size: 0,
        }
    }
}

/// Per-instance data sent to GPU.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    pub position_scale: [f32; 4], // xyz = position, w = scale
    pub color: [f32; 4],          // rgba
}

impl InstanceData {
    pub fn new(position: Vec3, scale: f32, color: Color) -> Self {
        let rgba = color.to_linear().to_f32_array();
        Self {
            position_scale: [position.x, position.y, position.z, scale],
            color: rgba,
        }
    }
}

/// Custom material for instanced rendering.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct InstancedMaterial {
    #[uniform(0)]
    pub base_color: LinearRgba,
}

impl Material for InstancedMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/instancing.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/instancing.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add instance data vertex buffer layout
        let instance_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // i_pos_scale at location 5
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                // i_color at location 6
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 6,
                },
            ],
        };

        descriptor
            .vertex
            .buffers
            .push(instance_layout);

        Ok(())
    }
}

/// Marker component for instanced mesh batches.
#[derive(Component)]
pub struct InstancedBatch {
    pub instance_count: usize,
}

/// Setup function to create 100k instanced cubes.
fn setup_instanced_cubes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<InstancingConfig>,
    terrain_config: Res<TerrainConfig>,
) {
    info!("Setting up {} instanced cubes...", config.instance_count);

    // Terrain with Perlin noise height variation
    let terrain_mesh = generate_terrain_mesh(&terrain_config);
    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.3, 0.15),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Terrain,
    ));

    // Use standard material with Bevy's automatic instancing
    // Bevy batches entities with identical mesh + material automatically
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Create a few material variations for visual variety
    let material_variants: Vec<Handle<StandardMaterial>> = [
        Color::srgb(0.8, 0.4, 0.2),  // Orange - residential
        Color::srgb(0.3, 0.5, 0.8),  // Blue - commercial
        Color::srgb(0.6, 0.6, 0.6),  // Gray - industrial
        Color::srgb(0.2, 0.7, 0.3),  // Green - parks
    ]
    .iter()
    .map(|&color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        })
    })
    .collect();

    let grid_size = config.grid_size;
    let spacing = 3.0;
    let half_size = (grid_size as f32 * spacing) / 2.0;

    // Spawn instances
    let mut count = 0;
    for x in 0..grid_size {
        for z in 0..grid_size {
            if count >= config.instance_count {
                break;
            }

            let world_x = (x as f32 * spacing) - half_size;
            let world_z = (z as f32 * spacing) - half_size;

            // Vary height based on position (simulate different building heights)
            let height = 1.0 + ((x + z) % 5) as f32 * 2.0;

            // Select material based on zone (simple pattern)
            let zone = ((x / 20) + (z / 20)) % 4;
            let material = material_variants[zone].clone();

            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(world_x, height / 2.0, world_z)
                    .with_scale(Vec3::new(1.0, height, 1.0)),
            ));

            count += 1;
        }
    }

    info!("Spawned {} building instances", count);

    // Note: Lighting is handled by day_night.rs for day/night cycle
}

/// Marker component for terrain entity.
#[derive(Component)]
pub struct Terrain;

/// Generate a terrain mesh with Perlin noise height variation.
fn generate_terrain_mesh(config: &TerrainConfig) -> Mesh {
    let perlin = Perlin::new(config.seed);
    let res = config.resolution as usize;
    let half_size = config.size / 2.0;
    let step = config.size / config.resolution as f32;

    // Generate vertices with noise-based height
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((res + 1) * (res + 1));
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((res + 1) * (res + 1));
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((res + 1) * (res + 1));

    for z in 0..=res {
        for x in 0..=res {
            let world_x = (x as f32 * step) - half_size;
            let world_z = (z as f32 * step) - half_size;

            // Sample fractal Perlin noise (multiple octaves)
            let height = sample_terrain_height(
                &perlin,
                world_x,
                world_z,
                config.noise_scale,
                config.height_scale,
                config.octaves,
            );

            positions.push([world_x, height, world_z]);
            normals.push([0.0, 1.0, 0.0]); // Will be recalculated
            uvs.push([x as f32 / res as f32, z as f32 / res as f32]);
        }
    }

    // Recalculate normals based on neighboring vertices
    for z in 0..=res {
        for x in 0..=res {
            let idx = z * (res + 1) + x;

            // Get heights of neighbors (with bounds checking)
            let h_left = if x > 0 { positions[idx - 1][1] } else { positions[idx][1] };
            let h_right = if x < res { positions[idx + 1][1] } else { positions[idx][1] };
            let h_up = if z > 0 { positions[idx - (res + 1)][1] } else { positions[idx][1] };
            let h_down = if z < res { positions[idx + (res + 1)][1] } else { positions[idx][1] };

            // Calculate normal from height differences
            let normal = Vec3::new(
                (h_left - h_right) / (2.0 * step),
                1.0,
                (h_up - h_down) / (2.0 * step),
            )
            .normalize();

            normals[idx] = normal.to_array();
        }
    }

    // Generate indices for triangles
    let mut indices: Vec<u32> = Vec::with_capacity(res * res * 6);
    for z in 0..res {
        for x in 0..res {
            let top_left = (z * (res + 1) + x) as u32;
            let top_right = top_left + 1;
            let bottom_left = ((z + 1) * (res + 1) + x) as u32;
            let bottom_right = bottom_left + 1;

            // Two triangles per quad (CCW winding)
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Sample terrain height using fractal Perlin noise.
fn sample_terrain_height(
    perlin: &Perlin,
    x: f32,
    z: f32,
    scale: f32,
    height_scale: f32,
    octaves: u32,
) -> f32 {
    let mut height = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = scale;
    let mut max_amplitude = 0.0;

    for _ in 0..octaves {
        let sample_x = x as f64 * frequency as f64;
        let sample_z = z as f64 * frequency as f64;
        height += perlin.get([sample_x, sample_z]) as f32 * amplitude;
        max_amplitude += amplitude;
        amplitude *= 0.5; // Each octave has half the amplitude
        frequency *= 2.0; // Each octave has double the frequency
    }

    // Normalize and scale
    (height / max_amplitude) * height_scale
}

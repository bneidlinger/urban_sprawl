//! Hardware instancing for rendering 100k+ entities efficiently.
//!
//! Uses Bevy's built-in instancing with custom instance data.

use bevy::{
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, VertexBufferLayout, VertexFormat,
            VertexStepMode,
        },
    },
    pbr::{MaterialPipeline, MaterialPipelineKey},
};
use bytemuck::{Pod, Zeroable};

pub struct InstancingPlugin;

impl Plugin for InstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<InstancedMaterial>::default())
            .init_resource::<InstancingConfig>()
            .add_systems(Startup, setup_instanced_cubes);
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
) {
    info!("Setting up {} instanced cubes...", config.instance_count);

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2000.0, 2000.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.3, 0.15),
            perceptual_roughness: 0.9,
            ..default()
        })),
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

    // Lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: false, // Disable shadows for performance with 100k objects
            ..default()
        },
        Transform::from_xyz(100.0, 200.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
}

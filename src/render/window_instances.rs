//! Window instance buffer management for GPU-driven window rendering.
//!
//! Batches hundreds of thousands of window quads into a single draw call
//! using hardware instancing. Each window has position, size, facing direction,
//! and night lighting properties.

#![allow(dead_code)]

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

use crate::procgen::building_factory::FacadeStyle;

/// Per-instance data for a window quad (64 bytes).
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct WindowInstanceData {
    /// Window center position in world space (xyz) + occupied flag (w)
    pub position_occupied: [f32; 4],    // 16 bytes
    /// Window size (width, height) + normal direction (x, z)
    pub size_normal: [f32; 4],          // 16 bytes
    /// Night light color (RGBA linear)
    pub color: [f32; 4],                // 16 bytes
    /// Intensity, facade type, metallic, roughness
    pub material_params: [f32; 4],      // 16 bytes
}

impl WindowInstanceData {
    pub fn new(
        position: Vec3,
        occupied: bool,
        size: Vec2,
        normal: Vec2,
        color: LinearRgba,
        intensity: f32,
        facade: FacadeStyle,
        metallic: f32,
        roughness: f32,
    ) -> Self {
        Self {
            position_occupied: [position.x, position.y, position.z, if occupied { 1.0 } else { 0.0 }],
            size_normal: [size.x, size.y, normal.x, normal.y],
            color: color.to_f32_array(),
            material_params: [intensity, facade as u32 as f32, metallic, roughness],
        }
    }

    /// Get the window position.
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.position_occupied[0], self.position_occupied[1], self.position_occupied[2])
    }

    /// Check if window is occupied (will light up at night).
    pub fn is_occupied(&self) -> bool {
        self.position_occupied[3] > 0.5
    }

    /// Get window size.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.size_normal[0], self.size_normal[1])
    }

    /// Get facing normal (XZ plane).
    pub fn normal(&self) -> Vec2 {
        Vec2::new(self.size_normal[2], self.size_normal[3])
    }

    /// Get the light intensity.
    pub fn intensity(&self) -> f32 {
        self.material_params[0]
    }

    /// Get the facade type.
    pub fn facade_type(&self) -> u32 {
        self.material_params[1] as u32
    }
}

/// Buffer containing all window instances for GPU rendering.
#[derive(Resource, Default)]
pub struct WindowInstanceBuffer {
    /// All window instance data
    instances: Vec<WindowInstanceData>,
    /// Dirty flag for GPU buffer updates
    pub dirty: bool,
}

impl WindowInstanceBuffer {
    /// Create a new empty buffer with capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            dirty: false,
        }
    }

    /// Add a window instance.
    pub fn push(&mut self, instance: WindowInstanceData) {
        self.instances.push(instance);
        self.dirty = true;
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
    }

    /// Get the number of instances.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Get all instances as a slice.
    pub fn instances(&self) -> &[WindowInstanceData] {
        &self.instances
    }

    /// Get raw bytes for GPU upload.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }

    /// Get instances by facade type for batched rendering.
    pub fn instances_by_facade(&self, facade: FacadeStyle) -> Vec<&WindowInstanceData> {
        let facade_id = facade as u32 as f32;
        self.instances
            .iter()
            .filter(|w| (w.material_params[1] - facade_id).abs() < 0.5)
            .collect()
    }
}

/// Statistics about window instances.
#[derive(Debug, Clone)]
pub struct WindowStats {
    pub total_windows: usize,
    pub occupied_windows: usize,
    pub by_facade: [(FacadeStyle, usize); 5],
}

impl Default for WindowStats {
    fn default() -> Self {
        Self {
            total_windows: 0,
            occupied_windows: 0,
            by_facade: [
                (FacadeStyle::Glass, 0),
                (FacadeStyle::Brick, 0),
                (FacadeStyle::Concrete, 0),
                (FacadeStyle::Metal, 0),
                (FacadeStyle::Painted, 0),
            ],
        }
    }
}

impl WindowInstanceBuffer {
    /// Calculate statistics about the window buffer.
    pub fn calculate_stats(&self) -> WindowStats {
        let mut stats = WindowStats::default();
        stats.total_windows = self.instances.len();

        let mut facade_counts = [0usize; 5];

        for window in &self.instances {
            if window.is_occupied() {
                stats.occupied_windows += 1;
            }

            let facade_idx = window.facade_type().min(4) as usize;
            facade_counts[facade_idx] += 1;
        }

        stats.by_facade = [
            (FacadeStyle::Glass, facade_counts[0]),
            (FacadeStyle::Brick, facade_counts[1]),
            (FacadeStyle::Concrete, facade_counts[2]),
            (FacadeStyle::Metal, facade_counts[3]),
            (FacadeStyle::Painted, facade_counts[4]),
        ];

        stats
    }
}

/// Custom material for instanced window rendering.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct WindowInstancedMaterial {
    /// Base glass color (tinted by per-instance color at night)
    #[uniform(0)]
    pub base_color: LinearRgba,
    /// Time of day for night factor calculation (0.0 = midnight, 0.5 = noon)
    #[uniform(1)]
    pub time_of_day: f32,
    /// Night factor (0.0 = day, 1.0 = night) - precomputed for efficiency
    #[uniform(2)]
    pub night_factor: f32,
}

impl Default for WindowInstancedMaterial {
    fn default() -> Self {
        Self {
            base_color: LinearRgba::new(0.2, 0.25, 0.3, 0.7),
            time_of_day: 0.5,
            night_factor: 0.0,
        }
    }
}

impl Material for WindowInstancedMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/window_instanced.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/window_instanced.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Window instance data layout (64 bytes per instance)
        let instance_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<WindowInstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // position_occupied at location 5
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                // size_normal at location 6
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 6,
                },
                // color at location 7
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 7,
                },
                // material_params at location 8
                bevy::render::render_resource::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 8,
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

/// Marker component for the window instance batch entity.
#[derive(Component)]
pub struct WindowInstanceBatch {
    pub instance_count: usize,
}

/// Plugin for window instance management.
pub struct WindowInstancesPlugin;

impl Plugin for WindowInstancesPlugin {
    fn build(&self, app: &mut App) {
        // WindowInstancedMaterial disabled - shader needs proper Bevy Material integration
        // TODO: Fix window_instanced.wgsl to align with Bevy's material binding expectations
        // app.add_plugins(MaterialPlugin::<WindowInstancedMaterial>::default())
        app.init_resource::<WindowInstanceBuffer>();
    }
}

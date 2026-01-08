//! GPU culling compute pipeline and render graph integration.
//!
//! Sets up the compute shader pipeline for frustum and HZB occlusion culling,
//! and integrates with Bevy's render graph for proper execution ordering.

use bevy::prelude::*;
use bevy::render::{
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    Extract, Render, RenderApp, RenderSet,
};
use bytemuck::{Pod, Zeroable};

use super::frustum::FrustumPlanes;
use super::object_data::{ObjectData, DrawIndexedIndirect};
use crate::render::hzb::HzbConfig;

/// Label for the GPU culling render graph node.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct GpuCullingLabel;

/// Uniforms passed to the culling compute shader.
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct CullUniforms {
    /// Number of objects to process
    pub object_count: u32,
    /// Padding added to bounding spheres for conservative culling
    pub frustum_padding: f32,
    /// 1 = HZB occlusion enabled, 0 = frustum only
    pub hzb_enabled: u32,
    /// Minimum screen size (pixels) to test against HZB
    pub min_screen_size: f32,
}

impl Default for CullUniforms {
    fn default() -> Self {
        Self {
            object_count: 0,
            frustum_padding: 0.0,
            hzb_enabled: 0,
            min_screen_size: 4.0,
        }
    }
}

/// GPU-side buffers for the culling system.
#[derive(Resource)]
pub struct GpuCullingBuffers {
    /// Uniform buffer for culling parameters
    pub uniforms_buffer: Buffer,
    /// Storage buffer for frustum planes
    pub frustum_buffer: Buffer,
    /// Storage buffer for object data (input)
    pub object_buffer: Buffer,
    /// Storage buffer for visibility results (output)
    pub visibility_buffer: Buffer,
    /// Storage buffer for indirect draw commands (output)
    pub indirect_buffer: Buffer,
    /// Bind group for the culling shader
    pub bind_group: Option<BindGroup>,
    /// Maximum number of objects the buffers can hold
    pub max_objects: u32,
    /// Whether buffers need recreation (size changed)
    pub dirty: bool,
}

impl GpuCullingBuffers {
    /// Create new GPU buffers with the given capacity.
    pub fn new(device: &RenderDevice, max_objects: u32) -> Self {
        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("cull_uniforms_buffer"),
            size: std::mem::size_of::<CullUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frustum_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("frustum_planes_buffer"),
            size: std::mem::size_of::<FrustumPlanes>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Object data buffer (read-only storage)
        let object_buffer_size = (std::mem::size_of::<ObjectData>() * max_objects as usize) as u64;
        let object_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("object_data_buffer"),
            size: object_buffer_size.max(32), // Minimum 32 bytes
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Visibility buffer (read-write storage, one u32 per object)
        let visibility_buffer_size = (std::mem::size_of::<u32>() * max_objects as usize) as u64;
        let visibility_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("visibility_buffer"),
            size: visibility_buffer_size.max(4), // Minimum 4 bytes
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Indirect draw buffer (read-write storage)
        let indirect_buffer_size =
            (std::mem::size_of::<DrawIndexedIndirect>() * max_objects as usize) as u64;
        let indirect_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("indirect_draw_buffer"),
            size: indirect_buffer_size.max(20), // Minimum one command (20 bytes)
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms_buffer,
            frustum_buffer,
            object_buffer,
            visibility_buffer,
            indirect_buffer,
            bind_group: None,
            max_objects,
            dirty: true,
        }
    }

    /// Resize buffers if needed.
    pub fn resize_if_needed(&mut self, device: &RenderDevice, new_max: u32) {
        if new_max > self.max_objects {
            // Recreate buffers with larger size
            let new_max = (new_max as f32 * 1.5) as u32; // Over-allocate by 50%
            *self = Self::new(device, new_max);
        }
    }
}

/// The GPU culling compute pipeline.
#[derive(Resource)]
pub struct GpuCullingPipeline {
    /// Bind group layout for the culling shader
    pub bind_group_layout: BindGroupLayout,
    /// The compute pipeline
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for GpuCullingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            "gpu_culling_bind_group_layout",
            &[
                // Binding 0: Uniforms
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            std::mem::size_of::<CullUniforms>() as u64
                        ),
                    },
                    count: None,
                },
                // Binding 1: Frustum planes (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            std::mem::size_of::<FrustumPlanes>() as u64
                        ),
                    },
                    count: None,
                },
                // Binding 2: Object data (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            std::mem::size_of::<ObjectData>() as u64
                        ),
                    },
                    count: None,
                },
                // Binding 3: Visibility output (storage, read-write)
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4), // At least one u32
                    },
                    count: None,
                },
                // Binding 4: Indirect draw commands (storage, read-write)
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            std::mem::size_of::<DrawIndexedIndirect>() as u64
                        ),
                    },
                    count: None,
                },
            ],
        );

        // Create compute pipeline
        let shader = world.load_asset("shaders/frustum_cull.wgsl");

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("gpu_culling_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader,
            shader_defs: vec![],
            entry_point: "main_indirect".into(),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}

/// Extracted data from the main world for GPU culling.
#[derive(Resource, Default)]
pub struct ExtractedCullData {
    /// Culling uniforms
    pub uniforms: CullUniforms,
    /// Frustum planes
    pub frustum: FrustumPlanes,
    /// Object data to upload
    pub objects: Vec<ObjectData>,
    /// Base indirect draw commands (to be modified by compute shader)
    pub indirect_commands: Vec<DrawIndexedIndirect>,
    /// Whether culling is enabled
    pub enabled: bool,
}

/// Plugin for GPU culling pipeline.
pub struct GpuCullingPipelinePlugin;

impl Plugin for GpuCullingPipelinePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ExtractedCullData>()
            .add_systems(ExtractSchedule, extract_cull_data)
            .add_systems(
                Render,
                (
                    prepare_culling_buffers.in_set(RenderSet::Prepare),
                    queue_culling_bind_group.in_set(RenderSet::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Initialize pipeline (requires asset server to be ready)
        render_app.init_resource::<GpuCullingPipeline>();

        // Create initial buffers
        let render_device = render_app.world().resource::<RenderDevice>();
        let buffers = GpuCullingBuffers::new(render_device, 1024);
        render_app.insert_resource(buffers);
    }
}

/// Extract culling data from the main world to the render world.
fn extract_cull_data(
    mut extracted: ResMut<ExtractedCullData>,
    config: Extract<Res<super::GpuCullingConfig>>,
    frustum: Extract<Res<FrustumPlanes>>,
    object_buffer: Extract<Res<super::ObjectDataBuffer>>,
    hzb_config: Extract<Option<Res<HzbConfig>>>,
) {
    extracted.enabled = config.enabled;

    if !config.enabled {
        extracted.objects.clear();
        extracted.indirect_commands.clear();
        return;
    }

    // Copy frustum planes
    extracted.frustum = **frustum;

    // Copy object data
    extracted.objects.clear();
    extracted.objects.extend_from_slice(object_buffer.objects());

    // Set up uniforms
    let hzb_enabled = hzb_config
        .as_ref()
        .map(|c| c.enabled)
        .unwrap_or(false);

    extracted.uniforms = CullUniforms {
        object_count: extracted.objects.len() as u32,
        frustum_padding: config.frustum_padding,
        hzb_enabled: if hzb_enabled && config.hzb_enabled { 1 } else { 0 },
        min_screen_size: config.hzb_min_screen_size,
    };

    // Create base indirect draw commands (one per object for now)
    // In a real implementation, these would be grouped by mesh type
    let object_count = extracted.objects.len();
    extracted.indirect_commands.clear();
    extracted.indirect_commands.reserve(object_count);
    for i in 0..object_count {
        extracted.indirect_commands.push(DrawIndexedIndirect {
            index_count: 36, // Placeholder - would come from mesh data
            instance_count: 1, // Will be set to 0 or 1 by compute shader
            first_index: 0,
            vertex_offset: 0,
            first_instance: i as u32,
        });
    }
}

/// Prepare GPU buffers for culling.
fn prepare_culling_buffers(
    mut buffers: ResMut<GpuCullingBuffers>,
    extracted: Res<ExtractedCullData>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    if !extracted.enabled || extracted.objects.is_empty() {
        return;
    }

    // Resize buffers if needed
    buffers.resize_if_needed(&render_device, extracted.objects.len() as u32);

    // Upload uniforms
    render_queue.write_buffer(
        &buffers.uniforms_buffer,
        0,
        bytemuck::bytes_of(&extracted.uniforms),
    );

    // Upload frustum planes
    render_queue.write_buffer(
        &buffers.frustum_buffer,
        0,
        bytemuck::bytes_of(&extracted.frustum),
    );

    // Upload object data
    if !extracted.objects.is_empty() {
        render_queue.write_buffer(
            &buffers.object_buffer,
            0,
            bytemuck::cast_slice(&extracted.objects),
        );
    }

    // Upload base indirect draw commands
    if !extracted.indirect_commands.is_empty() {
        render_queue.write_buffer(
            &buffers.indirect_buffer,
            0,
            bytemuck::cast_slice(&extracted.indirect_commands),
        );
    }

    buffers.dirty = false;
}

/// Create the bind group for the culling shader.
fn queue_culling_bind_group(
    mut buffers: ResMut<GpuCullingBuffers>,
    pipeline: Res<GpuCullingPipeline>,
    render_device: Res<RenderDevice>,
    extracted: Res<ExtractedCullData>,
) {
    if !extracted.enabled || extracted.objects.is_empty() {
        buffers.bind_group = None;
        return;
    }

    let bind_group = render_device.create_bind_group(
        "gpu_culling_bind_group",
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: buffers.uniforms_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: buffers.frustum_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: buffers.object_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: buffers.visibility_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: buffers.indirect_buffer.as_entire_binding(),
            },
        ],
    );

    buffers.bind_group = Some(bind_group);
}

/// Render graph node that executes the GPU culling compute shader.
pub struct GpuCullingNode;

impl render_graph::Node for GpuCullingNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let Some(buffers) = world.get_resource::<GpuCullingBuffers>() else {
            return Ok(());
        };

        let Some(bind_group) = &buffers.bind_group else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let culling_pipeline = world.resource::<GpuCullingPipeline>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(culling_pipeline.pipeline) else {
            return Ok(());
        };

        let extracted = world.resource::<ExtractedCullData>();
        if extracted.objects.is_empty() {
            return Ok(());
        }

        // Calculate workgroup count (64 threads per workgroup)
        let workgroup_count = (extracted.objects.len() as u32 + 63) / 64;

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("gpu_culling_pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        Ok(())
    }
}

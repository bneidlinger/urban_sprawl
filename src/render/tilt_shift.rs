//! Tilt-shift post-processing effect for a miniature/diorama look.
//!
//! Creates depth-of-field blur at the top and bottom of the screen while
//! keeping the center in focus, simulating a tilt-shift lens.

use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, Operations,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};

const TILT_SHIFT_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x2f6b8a9c4e1d3b5a7f9e8c6d4b2a1e3f);

pub struct TiltShiftPlugin;

impl Plugin for TiltShiftPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TILT_SHIFT_SHADER_HANDLE,
            "../../assets/shaders/tilt_shift.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<TiltShiftSettings>()
            .init_resource::<TiltShiftConfig>()
            .add_plugins((
                ExtractComponentPlugin::<TiltShiftSettings>::default(),
                UniformComponentPlugin::<TiltShiftSettings>::default(),
            ))
            .add_systems(PostStartup, setup_tilt_shift)
            .add_systems(Update, update_tilt_shift_settings);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<TiltShiftNode>>(Core3d, TiltShiftLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    TiltShiftLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<TiltShiftPipeline>();
    }
}

/// Configuration resource for tilt-shift effect (user-facing).
#[derive(Resource, Clone)]
pub struct TiltShiftConfig {
    /// Enable/disable the effect.
    pub enabled: bool,
    /// Vertical center of the focus band (0.0 = bottom, 1.0 = top, 0.5 = center).
    pub focus_center: f32,
    /// Width of the sharp focus band (0.0 - 1.0).
    pub focus_width: f32,
    /// Maximum blur strength at edges.
    pub blur_amount: f32,
    /// Number of blur samples (higher = smoother but slower).
    pub blur_samples: i32,
    /// Saturation boost for miniature effect (1.0 = normal).
    pub saturation: f32,
}

impl Default for TiltShiftConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            focus_center: 0.5,
            focus_width: 0.25,
            blur_amount: 3.0,
            blur_samples: 8,
            saturation: 1.15,
        }
    }
}

/// Component attached to the camera for tilt-shift settings (extracted to GPU).
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType, Reflect)]
#[reflect(Component)]
pub struct TiltShiftSettings {
    /// Vertical center of focus (0.0-1.0).
    pub focus_center: f32,
    /// Width of focus band (0.0-1.0).
    pub focus_width: f32,
    /// Maximum blur radius in pixels.
    pub blur_amount: f32,
    /// Number of blur samples.
    pub blur_samples: i32,
    /// Saturation multiplier.
    pub saturation: f32,
    /// Padding for alignment.
    #[reflect(ignore)]
    pub _padding: f32,
}

impl Default for TiltShiftSettings {
    fn default() -> Self {
        Self {
            focus_center: 0.5,
            focus_width: 0.25,
            blur_amount: 3.0,
            blur_samples: 8,
            saturation: 1.15,
            _padding: 0.0,
        }
    }
}

/// Render label for the tilt-shift node.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct TiltShiftLabel;

/// Render node that applies the tilt-shift effect.
#[derive(Default)]
struct TiltShiftNode;

impl ViewNode for TiltShiftNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DynamicUniformIndex<TiltShiftSettings>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, settings_index): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let tilt_shift_pipeline = world.resource::<TiltShiftPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let settings_uniforms = world.resource::<ComponentUniforms<TiltShiftSettings>>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(tilt_shift_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "tilt_shift_bind_group",
            &tilt_shift_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &tilt_shift_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("tilt_shift_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

/// Pipeline resource for the tilt-shift effect.
#[derive(Resource)]
struct TiltShiftPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for TiltShiftPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "tilt_shift_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<TiltShiftSettings>(true),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("tilt_shift_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: TILT_SHIFT_SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba8UnormSrgb,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

/// System to add TiltShiftSettings to the camera on startup.
fn setup_tilt_shift(mut commands: Commands, config: Res<TiltShiftConfig>, cameras: Query<Entity, With<Camera3d>>) {
    for camera_entity in cameras.iter() {
        if config.enabled {
            commands.entity(camera_entity).insert(TiltShiftSettings {
                focus_center: config.focus_center,
                focus_width: config.focus_width,
                blur_amount: config.blur_amount,
                blur_samples: config.blur_samples,
                saturation: config.saturation,
                _padding: 0.0,
            });
            info!("Tilt-shift effect enabled on camera");
        }
    }
}

/// System to sync TiltShiftConfig changes to camera settings.
fn update_tilt_shift_settings(
    config: Res<TiltShiftConfig>,
    mut commands: Commands,
    mut cameras: Query<(Entity, Option<&mut TiltShiftSettings>), With<Camera3d>>,
) {
    if !config.is_changed() {
        return;
    }

    for (entity, settings) in cameras.iter_mut() {
        if config.enabled {
            let new_settings = TiltShiftSettings {
                focus_center: config.focus_center,
                focus_width: config.focus_width,
                blur_amount: config.blur_amount,
                blur_samples: config.blur_samples,
                saturation: config.saturation,
                _padding: 0.0,
            };
            if let Some(mut existing) = settings {
                *existing = new_settings;
            } else {
                commands.entity(entity).insert(new_settings);
            }
        } else if settings.is_some() {
            commands.entity(entity).remove::<TiltShiftSettings>();
        }
    }
}

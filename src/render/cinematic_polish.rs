//! Cinematic post-processing effects for film-like visuals.
//!
//! Includes film grain, vignette, and chromatic aberration in a single
//! efficient shader pass for authentic cinematic look.

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

use crate::render::day_night::TimeOfDay;
use crate::render::tilt_shift::TiltShiftLabel;

const CINEMATIC_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x3a7c9e1f5b2d4a6c8e0f1a2b3c4d5e6f);

pub struct CinematicPolishPlugin;

impl Plugin for CinematicPolishPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CINEMATIC_SHADER_HANDLE,
            "../../assets/shaders/cinematic_polish.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<CinematicSettings>()
            .init_resource::<CinematicPolishConfig>()
            .add_plugins((
                ExtractComponentPlugin::<CinematicSettings>::default(),
                UniformComponentPlugin::<CinematicSettings>::default(),
            ))
            .add_systems(PostStartup, setup_cinematic_polish)
            .add_systems(Update, update_cinematic_settings);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<CinematicPolishNode>>(Core3d, CinematicPolishLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    // Run after tilt-shift, before end of post-processing
                    TiltShiftLabel,
                    CinematicPolishLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<CinematicPolishPipeline>();
    }
}

/// Configuration resource for cinematic effects (user-facing).
#[derive(Resource, Clone)]
pub struct CinematicPolishConfig {
    /// Master enable/disable.
    pub enabled: bool,

    // Film grain settings
    /// Enable film grain effect.
    pub grain_enabled: bool,
    /// Grain intensity (0.0 - 0.15, subtle to heavy).
    pub grain_intensity: f32,
    /// Grain size multiplier (1.0 - 3.0).
    pub grain_size: f32,

    // Vignette settings
    /// Enable vignette effect.
    pub vignette_enabled: bool,
    /// Vignette darkness intensity (0.0 - 1.0).
    pub vignette_intensity: f32,
    /// Vignette radius - where darkening starts (0.5 - 1.0).
    pub vignette_radius: f32,
    /// Vignette softness - transition smoothness (0.2 - 0.8).
    pub vignette_softness: f32,

    // Chromatic aberration settings
    /// Enable chromatic aberration.
    pub chromatic_enabled: bool,
    /// Chromatic aberration intensity (0.0 - 0.02).
    pub chromatic_intensity: f32,

    // Time-of-day modulation
    /// Multiply grain by this at night.
    pub night_grain_multiplier: f32,
    /// Multiply vignette by this at night.
    pub night_vignette_multiplier: f32,
}

impl Default for CinematicPolishConfig {
    fn default() -> Self {
        Self {
            enabled: true,

            grain_enabled: true,
            grain_intensity: 0.015,  // Subtle grain (was 0.04)
            grain_size: 1.0,         // Finer grain (was 1.5)

            vignette_enabled: true,
            vignette_intensity: 0.2,
            vignette_radius: 0.75,
            vignette_softness: 0.45,

            chromatic_enabled: true,
            chromatic_intensity: 0.002,

            night_grain_multiplier: 1.3,   // Subtle night boost (was 2.0)
            night_vignette_multiplier: 1.3,
        }
    }
}

/// Component attached to camera for cinematic settings (extracted to GPU).
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType, Reflect)]
#[reflect(Component)]
pub struct CinematicSettings {
    /// Film grain intensity (0.0 = off).
    pub grain_intensity: f32,
    /// Film grain size multiplier.
    pub grain_size: f32,
    /// Vignette intensity (0.0 = off).
    pub vignette_intensity: f32,
    /// Vignette radius.
    pub vignette_radius: f32,
    /// Vignette softness.
    pub vignette_softness: f32,
    /// Chromatic aberration intensity (0.0 = off).
    pub chromatic_intensity: f32,
    /// Time value for grain animation.
    pub time: f32,
    /// Padding for alignment.
    #[reflect(ignore)]
    pub _padding: f32,
}

impl Default for CinematicSettings {
    fn default() -> Self {
        Self {
            grain_intensity: 0.015,
            grain_size: 1.0,
            vignette_intensity: 0.2,
            vignette_radius: 0.75,
            vignette_softness: 0.45,
            chromatic_intensity: 0.002,
            time: 0.0,
            _padding: 0.0,
        }
    }
}

/// Render label for the cinematic polish node.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CinematicPolishLabel;

/// Render node that applies cinematic post-processing effects.
#[derive(Default)]
struct CinematicPolishNode;

impl ViewNode for CinematicPolishNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DynamicUniformIndex<CinematicSettings>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, settings_index): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let cinematic_pipeline = world.resource::<CinematicPolishPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let settings_uniforms = world.resource::<ComponentUniforms<CinematicSettings>>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(cinematic_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "cinematic_polish_bind_group",
            &cinematic_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &cinematic_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("cinematic_polish_pass"),
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

/// Pipeline resource for the cinematic polish effect.
#[derive(Resource)]
struct CinematicPolishPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for CinematicPolishPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "cinematic_polish_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<CinematicSettings>(true),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("cinematic_polish_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: CINEMATIC_SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float,
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

/// System to add CinematicSettings to the camera on startup.
fn setup_cinematic_polish(
    mut commands: Commands,
    config: Res<CinematicPolishConfig>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    for camera_entity in cameras.iter() {
        if config.enabled {
            commands.entity(camera_entity).insert(CinematicSettings {
                grain_intensity: if config.grain_enabled { config.grain_intensity } else { 0.0 },
                grain_size: config.grain_size,
                vignette_intensity: if config.vignette_enabled { config.vignette_intensity } else { 0.0 },
                vignette_radius: config.vignette_radius,
                vignette_softness: config.vignette_softness,
                chromatic_intensity: if config.chromatic_enabled { config.chromatic_intensity } else { 0.0 },
                time: 0.0,
                _padding: 0.0,
            });
            info!("Cinematic polish effects enabled on camera");
        }
    }
}

/// System to sync config changes and time-of-day modulation.
fn update_cinematic_settings(
    config: Res<CinematicPolishConfig>,
    time_of_day: Option<Res<TimeOfDay>>,
    time: Res<Time>,
    mut commands: Commands,
    mut cameras: Query<(Entity, Option<&mut CinematicSettings>), With<Camera3d>>,
) {
    // Calculate night factor (0.0 = day, 1.0 = night)
    let night_factor: f32 = if let Some(tod) = &time_of_day {
        // Night is roughly 20:00 - 6:00
        let hour = tod.hour();
        if hour >= 20.0 || hour < 6.0 {
            if hour >= 20.0 {
                ((hour - 20.0) / 2.0).min(1.0) // Fade in from 20:00 to 22:00
            } else if hour < 4.0 {
                1.0 // Full night
            } else {
                ((6.0 - hour) / 2.0).max(0.0) // Fade out from 4:00 to 6:00
            }
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Calculate modulated intensities
    let grain_mod = 1.0 + (config.night_grain_multiplier - 1.0) * night_factor;
    let vignette_mod = 1.0 + (config.night_vignette_multiplier - 1.0) * night_factor;

    for (entity, settings) in cameras.iter_mut() {
        if config.enabled {
            let new_settings = CinematicSettings {
                grain_intensity: if config.grain_enabled {
                    config.grain_intensity * grain_mod
                } else {
                    0.0
                },
                grain_size: config.grain_size,
                vignette_intensity: if config.vignette_enabled {
                    config.vignette_intensity * vignette_mod
                } else {
                    0.0
                },
                vignette_radius: config.vignette_radius,
                vignette_softness: config.vignette_softness,
                chromatic_intensity: if config.chromatic_enabled {
                    config.chromatic_intensity
                } else {
                    0.0
                },
                time: time.elapsed_secs(),
                _padding: 0.0,
            };

            if let Some(mut existing) = settings {
                *existing = new_settings;
            } else {
                commands.entity(entity).insert(new_settings);
            }
        } else if settings.is_some() {
            commands.entity(entity).remove::<CinematicSettings>();
        }
    }
}

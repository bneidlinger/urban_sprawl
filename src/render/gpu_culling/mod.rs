//! GPU-driven frustum culling system with HZB occlusion.
//!
//! Uses compute shaders to cull objects against the view frustum and HZB depth
//! pyramid before rendering. This offloads culling from the CPU to the GPU,
//! enabling efficient handling of 100,000+ objects with minimal CPU overhead.
//!
//! ## Architecture
//!
//! 1. **ObjectData Buffer**: Contains bounding spheres and metadata for all cullable objects
//! 2. **Frustum Planes**: Extracted from the view-projection matrix each frame
//! 3. **HZB Pyramid**: Hierarchical depth buffer for occlusion testing
//! 4. **Visibility Buffer**: Output of culling - marks each object as visible or culled
//! 5. **Indirect Draw Buffer**: Optional - for fully GPU-driven indirect rendering
//!
//! ## Culling Order
//!
//! 1. Frustum culling (cheap, rejects ~50% of objects)
//! 2. HZB occlusion culling (more expensive, rejects occluded objects)
//!
//! ## Usage
//!
//! Objects register with the culling system by adding a `GpuCullable` component.
//! The system automatically extracts bounding spheres and updates the GPU buffers.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::camera::CameraProjection;

use super::hzb::{HzbConfig, HzbStats, project_sphere_to_screen};

pub mod frustum;
pub mod object_data;
pub mod pipeline;

pub use frustum::{FrustumPlanes, extract_frustum_planes};
pub use object_data::{ObjectData, ObjectDataBuffer, GpuCullable, CullStats, DrawIndexedIndirect, IndirectDrawBuffer};
pub use pipeline::{GpuCullingPipelinePlugin, GpuCullingBuffers, GpuCullingPipeline, CullUniforms};

/// Plugin for GPU-driven frustum culling.
pub struct GpuCullingPlugin;

impl Plugin for GpuCullingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GpuCullingConfig>()
            .init_resource::<ObjectDataBuffer>()
            .init_resource::<IndirectDrawBuffer>()
            .init_resource::<FrustumPlanes>()
            .init_resource::<CameraViewProj>()
            .init_resource::<CullStats>()
            // GPU pipeline temporarily disabled - using CPU fallback
            // TODO: Re-enable after fixing render pipeline compatibility
            // .add_plugins(GpuCullingPipelinePlugin)
            .add_systems(
                PostUpdate,
                (
                    update_object_data_buffer,
                    extract_camera_frustum,
                    perform_cpu_culling, // CPU fallback until compute shaders are fully integrated
                    update_cull_stats,
                )
                    .chain(),
            );
    }
}

/// Configuration for GPU culling.
#[derive(Resource)]
pub struct GpuCullingConfig {
    /// Enable GPU culling (falls back to CPU if compute shaders unavailable)
    pub enabled: bool,
    /// Use CPU culling as fallback
    pub cpu_fallback: bool,
    /// Padding added to bounding spheres for conservative culling
    pub frustum_padding: f32,
    /// Maximum number of cullable objects
    pub max_objects: usize,
    /// Enable HZB occlusion culling
    pub hzb_enabled: bool,
    /// Minimum screen-space size (pixels) to test against HZB
    pub hzb_min_screen_size: f32,
    /// Depth bias for conservative HZB testing
    pub hzb_depth_bias: f32,
}

impl Default for GpuCullingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_fallback: true,
            frustum_padding: 0.0,
            max_objects: 100_000,
            hzb_enabled: true,
            hzb_min_screen_size: 4.0,
            hzb_depth_bias: 0.001,
        }
    }
}

/// Stores the current camera's view-projection matrix for culling operations.
#[derive(Resource, Default)]
pub struct CameraViewProj {
    /// Combined view-projection matrix
    pub view_proj: Mat4,
    /// Screen dimensions
    pub screen_size: Vec2,
    /// Near plane distance
    pub near: f32,
    /// Far plane distance
    pub far: f32,
}

/// Update the object data buffer with all cullable entities.
fn update_object_data_buffer(
    config: Res<GpuCullingConfig>,
    mut buffer: ResMut<ObjectDataBuffer>,
    query: Query<(Entity, &GlobalTransform, &GpuCullable)>,
) {
    if !config.enabled {
        return;
    }

    buffer.clear();

    for (entity, transform, cullable) in query.iter() {
        // Calculate world-space bounding sphere
        let center = transform.translation();
        let scale = transform.to_scale_rotation_translation().0;
        let max_scale = scale.x.max(scale.y).max(scale.z);
        let world_radius = cullable.local_radius * max_scale;

        let bits = entity.to_bits();
        buffer.push(ObjectData {
            bounding_sphere: [center.x, center.y, center.z, world_radius],
            entity_bits_low: bits as u32,
            entity_bits_high: (bits >> 32) as u32,
            mesh_id: cullable.mesh_id,
            flags: if cullable.visible { 1 } else { 0 },
        });
    }

    buffer.mark_dirty();
}

/// Extract frustum planes from the main camera.
fn extract_camera_frustum(
    mut frustum: ResMut<FrustumPlanes>,
    mut camera_vp: ResMut<CameraViewProj>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>,
) {
    for (camera, transform, projection) in camera_query.iter() {
        if !camera.is_active {
            continue;
        }

        // Get view and projection matrices
        let view_matrix = transform.compute_matrix().inverse();

        let (projection_matrix, near, far) = match projection {
            Projection::Perspective(persp) => (
                persp.get_clip_from_view(),
                persp.near,
                persp.far,
            ),
            Projection::Orthographic(ortho) => (
                ortho.get_clip_from_view(),
                ortho.near,
                ortho.far,
            ),
        };

        // Combined view-projection matrix
        let view_proj = projection_matrix * view_matrix;

        // Extract frustum planes
        *frustum = extract_frustum_planes(view_proj);

        // Store view-proj matrix and camera info for HZB culling
        camera_vp.view_proj = view_proj;
        camera_vp.near = near;
        camera_vp.far = far;

        // Get screen size from camera viewport
        if let Some(viewport) = &camera.viewport {
            camera_vp.screen_size = Vec2::new(
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
            );
        } else {
            // Default to common resolution if viewport not set
            camera_vp.screen_size = Vec2::new(1920.0, 1080.0);
        }

        // Only process the first active camera
        break;
    }
}

/// CPU-based frustum culling (fallback when GPU culling is unavailable).
fn perform_cpu_culling(
    config: Res<GpuCullingConfig>,
    frustum: Res<FrustumPlanes>,
    mut query: Query<(&GlobalTransform, &mut GpuCullable, &mut Visibility)>,
) {
    if !config.enabled || !config.cpu_fallback {
        return;
    }

    for (transform, mut cullable, mut visibility) in query.iter_mut() {
        // Calculate world-space bounding sphere
        let center = transform.translation();
        let scale = transform.to_scale_rotation_translation().0;
        let max_scale = scale.x.max(scale.y).max(scale.z);
        let world_radius = cullable.local_radius * max_scale + config.frustum_padding;

        // Test against all frustum planes
        let is_visible = frustum.test_sphere(center, world_radius);

        cullable.visible = is_visible;

        // Update Bevy's visibility component
        *visibility = if is_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Resource to track when to log stats.
#[derive(Resource)]
struct CullStatsTimer(Timer);

impl Default for CullStatsTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

/// Update culling statistics.
fn update_cull_stats(
    time: Res<Time>,
    query: Query<&GpuCullable>,
    mut stats: ResMut<CullStats>,
    mut timer: Local<Option<CullStatsTimer>>,
) {
    // Initialize timer on first run
    if timer.is_none() {
        *timer = Some(CullStatsTimer::default());
    }

    let mut total = 0;
    let mut visible = 0;

    for cullable in query.iter() {
        total += 1;
        if cullable.visible {
            visible += 1;
        }
    }

    let prev_total = stats.total_objects;
    stats.total_objects = total;
    stats.visible_objects = visible;
    stats.culled_objects = total - visible;
    stats.cull_ratio = if total > 0 {
        (total - visible) as f32 / total as f32
    } else {
        0.0
    };

    // Log stats periodically or when total changes significantly
    if let Some(ref mut t) = *timer {
        t.0.tick(time.delta());
        if t.0.just_finished() && total > 0 {
            info!(
                "GPU Culling: {}/{} visible ({:.1}% culled)",
                visible, total,
                stats.cull_ratio * 100.0
            );
        }
    }

    // Also log once when buildings first appear
    if prev_total == 0 && total > 0 {
        info!(
            "GPU Culling initialized: {} cullable objects",
            total
        );
    }
}

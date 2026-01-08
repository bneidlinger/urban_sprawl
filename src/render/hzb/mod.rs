//! Hierarchical Z-Buffer (HZB) for occlusion culling.
//!
//! Generates a depth pyramid from the previous frame's depth buffer,
//! enabling efficient occlusion queries in the GPU culling pass.
//!
//! ## How It Works
//!
//! 1. After the depth prepass, we generate a mip chain of the depth buffer
//! 2. Each mip level contains the maximum depth of the 2x2 region below it
//! 3. During culling, we project object bounds to screen space
//! 4. We sample the HZB at the appropriate mip level based on projected size
//! 5. If the object's near depth > HZB depth, it's occluded
//!
//! ## Previous Frame Approach
//!
//! To avoid render dependencies, we use the previous frame's depth buffer.
//! This introduces minor artifacts for fast-moving cameras but works well
//! for typical city flyover scenarios.

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
    Render, RenderApp, RenderSet,
};

pub mod pyramid;

pub use pyramid::{HzbPyramid, HzbPyramidConfig, CpuHzbPyramid, calculate_mip_count};

/// Plugin for HZB-based occlusion culling.
pub struct HzbPlugin;

impl Plugin for HzbPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HzbConfig>()
            .init_resource::<HzbStats>()
            .add_plugins(ExtractResourcePlugin::<HzbConfig>::default())
            .add_systems(Update, update_hzb_stats);

        // Note: Full render graph integration would go in the render app
        // For now, we provide CPU-side resources and stats tracking
    }
}

/// Configuration for HZB generation.
#[derive(Resource, Clone, ExtractResource)]
pub struct HzbConfig {
    /// Enable HZB occlusion culling
    pub enabled: bool,
    /// Maximum number of mip levels to generate
    pub max_mip_levels: u32,
    /// Depth bias for conservative culling (objects slightly closer pass)
    pub depth_bias: f32,
    /// Screen-space threshold below which objects skip HZB test
    pub min_screen_size: f32,
    /// Use previous frame's depth (avoids render dependency)
    pub use_previous_frame: bool,
}

impl Default for HzbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_mip_levels: 10, // Supports up to 1024x1024 base resolution
            depth_bias: 0.001,
            min_screen_size: 4.0, // Objects smaller than 4 pixels skip HZB
            use_previous_frame: true,
        }
    }
}

/// Statistics for HZB occlusion culling.
#[derive(Resource, Default, Debug)]
pub struct HzbStats {
    /// Number of objects tested against HZB
    pub objects_tested: usize,
    /// Number of objects culled by HZB (occluded)
    pub objects_occluded: usize,
    /// Number of objects that passed frustum but failed HZB
    pub occlusion_ratio: f32,
    /// Current HZB pyramid resolution
    pub pyramid_resolution: UVec2,
    /// Number of mip levels in pyramid
    pub mip_levels: u32,
}

impl HzbStats {
    /// Get a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "HZB: {}/{} occluded ({:.1}%), {}x{} pyramid ({} mips)",
            self.objects_occluded,
            self.objects_tested,
            self.occlusion_ratio * 100.0,
            self.pyramid_resolution.x,
            self.pyramid_resolution.y,
            self.mip_levels
        )
    }
}

/// Update HZB statistics (placeholder - real stats come from GPU readback).
fn update_hzb_stats(
    config: Res<HzbConfig>,
    mut stats: ResMut<HzbStats>,
) {
    if !config.enabled {
        stats.objects_tested = 0;
        stats.objects_occluded = 0;
        stats.occlusion_ratio = 0.0;
        return;
    }

    // Real implementation would read back from GPU
    // For now, stats are updated by the culling system
}

/// Data needed for HZB occlusion testing in shaders.
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct HzbUniforms {
    /// Inverse projection matrix for unprojecting depth
    pub inv_projection: [[f32; 4]; 4],
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Number of mip levels
    pub mip_count: u32,
    /// Depth bias for conservative culling
    pub depth_bias: f32,
    /// Near plane distance
    pub near_plane: f32,
    /// Far plane distance
    pub far_plane: f32,
    /// Padding
    pub _padding: [f32; 2],
}

impl Default for HzbUniforms {
    fn default() -> Self {
        Self {
            inv_projection: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            screen_size: [1920.0, 1080.0],
            mip_count: 10,
            depth_bias: 0.001,
            near_plane: 0.1,
            far_plane: 1000.0,
            _padding: [0.0; 2],
        }
    }
}

impl HzbUniforms {
    /// Create uniforms from camera projection.
    pub fn from_projection(
        projection: Mat4,
        screen_width: f32,
        screen_height: f32,
        mip_count: u32,
        near: f32,
        far: f32,
        depth_bias: f32,
    ) -> Self {
        let inv = projection.inverse();
        Self {
            inv_projection: [
                [inv.x_axis.x, inv.x_axis.y, inv.x_axis.z, inv.x_axis.w],
                [inv.y_axis.x, inv.y_axis.y, inv.y_axis.z, inv.y_axis.w],
                [inv.z_axis.x, inv.z_axis.y, inv.z_axis.z, inv.z_axis.w],
                [inv.w_axis.x, inv.w_axis.y, inv.w_axis.z, inv.w_axis.w],
            ],
            screen_size: [screen_width, screen_height],
            mip_count,
            depth_bias,
            near_plane: near,
            far_plane: far,
            _padding: [0.0; 2],
        }
    }
}

/// CPU-side occlusion testing (fallback when GPU HZB unavailable).
/// Tests if an object's bounding sphere is potentially occluded.
pub fn cpu_occlusion_test(
    sphere_center: Vec3,
    sphere_radius: f32,
    view_proj: Mat4,
    depth_buffer: &[f32],
    width: u32,
    height: u32,
) -> bool {
    // Project sphere center to screen space
    let clip = view_proj * sphere_center.extend(1.0);

    // Behind camera check
    if clip.w <= 0.0 {
        return false; // Not occluded, but culled by frustum
    }

    let ndc = clip.xyz() / clip.w;

    // Outside NDC bounds
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 {
        return false; // Not occluded, but culled by frustum
    }

    // Convert to screen coordinates
    let screen_x = ((ndc.x + 1.0) * 0.5 * width as f32) as u32;
    let screen_y = ((1.0 - ndc.y) * 0.5 * height as f32) as u32;

    // Clamp to valid range
    let screen_x = screen_x.min(width - 1);
    let screen_y = screen_y.min(height - 1);

    // Get depth at screen position
    let idx = (screen_y * width + screen_x) as usize;
    if idx >= depth_buffer.len() {
        return false;
    }

    let buffer_depth = depth_buffer[idx];

    // Calculate object's near depth (front of bounding sphere)
    // In clip space, depth is stored as z/w in [0, 1] for Vulkan
    let object_depth = ndc.z;

    // Object is occluded if its near depth is greater than buffer depth
    object_depth > buffer_depth
}

/// Calculate the appropriate mip level for sampling HZB.
/// Based on projected screen-space size of the object.
pub fn calculate_hzb_mip_level(
    screen_size_pixels: f32,
    pyramid_base_size: f32,
    max_mip: u32,
) -> u32 {
    if screen_size_pixels <= 0.0 {
        return max_mip;
    }

    // We want to sample a mip level where one texel covers roughly
    // the same area as the object's screen projection
    let ideal_mip = (pyramid_base_size / screen_size_pixels).log2().ceil() as u32;
    ideal_mip.min(max_mip)
}

/// Project a bounding sphere to screen space and return its size in pixels.
pub fn project_sphere_to_screen(
    center: Vec3,
    radius: f32,
    view_proj: Mat4,
    screen_width: f32,
    screen_height: f32,
) -> Option<f32> {
    // Project center
    let clip = view_proj * center.extend(1.0);
    if clip.w <= 0.0 {
        return None; // Behind camera
    }

    let ndc_center = clip.xyz() / clip.w;

    // Project a point at the edge of the sphere
    // Use the point that's most perpendicular to the view direction
    let view_dir = Vec3::new(
        view_proj.x_axis.z,
        view_proj.y_axis.z,
        view_proj.z_axis.z,
    ).normalize();

    // Find perpendicular direction in world space
    let up = if view_dir.y.abs() < 0.99 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let right = view_dir.cross(up).normalize();

    let edge_point = center + right * radius;
    let edge_clip = view_proj * edge_point.extend(1.0);

    if edge_clip.w <= 0.0 {
        return None;
    }

    let ndc_edge = edge_clip.xyz() / edge_clip.w;

    // Calculate screen-space distance
    let screen_center = Vec2::new(
        (ndc_center.x + 1.0) * 0.5 * screen_width,
        (1.0 - ndc_center.y) * 0.5 * screen_height,
    );
    let screen_edge = Vec2::new(
        (ndc_edge.x + 1.0) * 0.5 * screen_width,
        (1.0 - ndc_edge.y) * 0.5 * screen_height,
    );

    Some(screen_center.distance(screen_edge) * 2.0) // Diameter
}

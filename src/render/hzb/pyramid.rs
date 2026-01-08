//! Depth pyramid (HZB) generation and management.
//!
//! Creates a hierarchical depth buffer by downsampling the depth texture
//! using max reduction. Each mip level represents the maximum depth
//! in the corresponding 2x2 region of the level below.

use bevy::prelude::*;
use bevy::render::render_resource::*;
use bytemuck::{Pod, Zeroable};

/// Configuration for the HZB pyramid dimensions.
#[derive(Resource, Clone)]
pub struct HzbPyramidConfig {
    /// Base resolution of the pyramid (usually matches render target)
    pub base_width: u32,
    pub base_height: u32,
    /// Maximum number of mip levels
    pub max_mip_levels: u32,
    /// Format for the depth pyramid texture
    pub format: TextureFormat,
}

impl Default for HzbPyramidConfig {
    fn default() -> Self {
        Self {
            base_width: 1920,
            base_height: 1080,
            max_mip_levels: 10,
            format: TextureFormat::R32Float,
        }
    }
}

impl HzbPyramidConfig {
    /// Create config from window size.
    pub fn from_window(width: u32, height: u32) -> Self {
        let max_mips = calculate_mip_count(width, height);
        Self {
            base_width: width,
            base_height: height,
            max_mip_levels: max_mips,
            format: TextureFormat::R32Float,
        }
    }

    /// Get the actual number of mip levels for current resolution.
    pub fn mip_count(&self) -> u32 {
        calculate_mip_count(self.base_width, self.base_height)
            .min(self.max_mip_levels)
    }

    /// Get the size of a specific mip level.
    pub fn mip_size(&self, level: u32) -> (u32, u32) {
        let width = (self.base_width >> level).max(1);
        let height = (self.base_height >> level).max(1);
        (width, height)
    }
}

/// Calculate the number of mip levels for a given resolution.
pub fn calculate_mip_count(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (max_dim as f32).log2().floor() as u32 + 1
}

/// The HZB depth pyramid resource.
#[derive(Resource)]
pub struct HzbPyramid {
    /// The pyramid texture with all mip levels
    pub texture: Option<Handle<Image>>,
    /// Current pyramid configuration
    pub config: HzbPyramidConfig,
    /// Previous frame's pyramid (for temporal stability)
    pub previous_frame: Option<Handle<Image>>,
    /// Whether the pyramid needs regeneration
    pub dirty: bool,
    /// Frame counter for double-buffering
    frame_index: u32,
}

impl Default for HzbPyramid {
    fn default() -> Self {
        Self {
            texture: None,
            config: HzbPyramidConfig::default(),
            previous_frame: None,
            dirty: true,
            frame_index: 0,
        }
    }
}

impl HzbPyramid {
    /// Create a new pyramid with the given configuration.
    pub fn new(config: HzbPyramidConfig) -> Self {
        Self {
            texture: None,
            config,
            previous_frame: None,
            dirty: true,
            frame_index: 0,
        }
    }

    /// Update configuration (marks pyramid as dirty).
    pub fn update_config(&mut self, width: u32, height: u32) {
        if self.config.base_width != width || self.config.base_height != height {
            self.config = HzbPyramidConfig::from_window(width, height);
            self.dirty = true;
        }
    }

    /// Get the current mip count.
    pub fn mip_count(&self) -> u32 {
        self.config.mip_count()
    }

    /// Get pyramid dimensions at base level.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.config.base_width, self.config.base_height)
    }

    /// Advance frame and swap buffers.
    pub fn advance_frame(&mut self) {
        self.frame_index = self.frame_index.wrapping_add(1);
        std::mem::swap(&mut self.texture, &mut self.previous_frame);
        self.dirty = true;
    }

    /// Get the texture to use for occlusion queries.
    /// Returns previous frame's pyramid for temporal stability.
    pub fn query_texture(&self) -> Option<&Handle<Image>> {
        self.previous_frame.as_ref().or(self.texture.as_ref())
    }
}

/// Uniforms for the HZB generation compute shader.
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct HzbGenerateUniforms {
    /// Input mip dimensions
    pub input_size: [u32; 2],
    /// Output mip dimensions
    pub output_size: [u32; 2],
    /// Source mip level
    pub src_mip: u32,
    /// Destination mip level
    pub dst_mip: u32,
    /// Padding
    pub _padding: [u32; 2],
}

impl HzbGenerateUniforms {
    pub fn new(input_width: u32, input_height: u32, output_width: u32, output_height: u32, src_mip: u32, dst_mip: u32) -> Self {
        Self {
            input_size: [input_width, input_height],
            output_size: [output_width, output_height],
            src_mip,
            dst_mip,
            _padding: [0; 2],
        }
    }
}

/// CPU-side HZB pyramid for software fallback.
#[derive(Default)]
pub struct CpuHzbPyramid {
    /// Mip levels stored as Vec of depth values
    pub mips: Vec<Vec<f32>>,
    /// Dimensions of each mip level
    pub dimensions: Vec<(u32, u32)>,
}

impl CpuHzbPyramid {
    /// Create from a depth buffer.
    pub fn from_depth_buffer(depth: &[f32], width: u32, height: u32) -> Self {
        let mip_count = calculate_mip_count(width, height);
        let mut mips = Vec::with_capacity(mip_count as usize);
        let mut dimensions = Vec::with_capacity(mip_count as usize);

        // Level 0 is the original depth buffer
        mips.push(depth.to_vec());
        dimensions.push((width, height));

        // Generate subsequent mip levels
        let mut current_width = width;
        let mut current_height = height;

        for _ in 1..mip_count {
            let prev_mip = mips.last().unwrap();
            let prev_width = current_width;
            let prev_height = current_height;

            current_width = (current_width / 2).max(1);
            current_height = (current_height / 2).max(1);

            let mut new_mip = vec![0.0f32; (current_width * current_height) as usize];

            for y in 0..current_height {
                for x in 0..current_width {
                    // Sample 2x2 region from previous level
                    let src_x = x * 2;
                    let src_y = y * 2;

                    let mut max_depth = f32::NEG_INFINITY;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let sx = (src_x + dx).min(prev_width - 1);
                            let sy = (src_y + dy).min(prev_height - 1);
                            let idx = (sy * prev_width + sx) as usize;
                            if idx < prev_mip.len() {
                                max_depth = max_depth.max(prev_mip[idx]);
                            }
                        }
                    }

                    new_mip[(y * current_width + x) as usize] = max_depth;
                }
            }

            mips.push(new_mip);
            dimensions.push((current_width, current_height));
        }

        Self { mips, dimensions }
    }

    /// Sample the HZB at a specific mip level and UV coordinate.
    pub fn sample(&self, mip: u32, u: f32, v: f32) -> f32 {
        let mip = (mip as usize).min(self.mips.len().saturating_sub(1));
        if self.mips.is_empty() {
            return 1.0;
        }

        let (width, height) = self.dimensions[mip];
        let x = ((u * width as f32) as u32).min(width - 1);
        let y = ((v * height as f32) as u32).min(height - 1);
        let idx = (y * width + x) as usize;

        self.mips[mip].get(idx).copied().unwrap_or(1.0)
    }

    /// Test if a screen-space rect is occluded.
    pub fn test_rect_occluded(
        &self,
        min_uv: Vec2,
        max_uv: Vec2,
        object_depth: f32,
        depth_bias: f32,
    ) -> bool {
        // Calculate appropriate mip level based on rect size
        let uv_size = max_uv - min_uv;
        let screen_size = uv_size.x.max(uv_size.y);

        if self.mips.is_empty() || self.dimensions.is_empty() {
            return false;
        }

        let base_size = self.dimensions[0].0.max(self.dimensions[0].1) as f32;
        let pixel_size = screen_size * base_size;
        let mip = super::calculate_hzb_mip_level(pixel_size, base_size, self.mips.len() as u32 - 1);

        // Sample at center of rect
        let center_u = (min_uv.x + max_uv.x) * 0.5;
        let center_v = (min_uv.y + max_uv.y) * 0.5;
        let hzb_depth = self.sample(mip, center_u, center_v);

        // Object is occluded if its depth is greater than HZB depth
        object_depth > hzb_depth + depth_bias
    }
}

// Note: GPU bind group layout functions moved to Phase 7 (Indirect Draw Integration)
// when the full compute pipeline is implemented. For now, we use CPU fallback.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mip_count_calculation() {
        assert_eq!(calculate_mip_count(1, 1), 1);
        assert_eq!(calculate_mip_count(2, 2), 2);
        assert_eq!(calculate_mip_count(4, 4), 3);
        assert_eq!(calculate_mip_count(1024, 1024), 11);
        assert_eq!(calculate_mip_count(1920, 1080), 11);
    }

    #[test]
    fn test_cpu_hzb_pyramid() {
        // Create a simple 4x4 depth buffer
        let depth = vec![
            0.1, 0.2, 0.3, 0.4,
            0.2, 0.3, 0.4, 0.5,
            0.3, 0.4, 0.5, 0.6,
            0.4, 0.5, 0.6, 0.7,
        ];

        let pyramid = CpuHzbPyramid::from_depth_buffer(&depth, 4, 4);

        // Should have 3 mip levels (4x4, 2x2, 1x1)
        assert_eq!(pyramid.mips.len(), 3);
        assert_eq!(pyramid.dimensions[0], (4, 4));
        assert_eq!(pyramid.dimensions[1], (2, 2));
        assert_eq!(pyramid.dimensions[2], (1, 1));

        // Level 1 should contain max of each 2x2 region
        // Top-left 2x2: max(0.1, 0.2, 0.2, 0.3) = 0.3
        assert!((pyramid.mips[1][0] - 0.3).abs() < 0.001);

        // Level 2 should contain max of entire buffer
        assert!((pyramid.mips[2][0] - 0.7).abs() < 0.001);
    }
}

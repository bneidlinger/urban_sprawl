//! Procedural facade texture generation and texture array management.
//!
//! Generates runtime textures for building facades without external assets.
//! Uses texture arrays for efficient single-draw-call rendering of varied materials.

#![allow(dead_code)]

use bevy::{
    prelude::*,
    image::Image,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

use crate::procgen::building_factory::FacadeStyle;

/// Size of each facade texture layer.
pub const TEXTURE_SIZE: u32 = 256;

/// Number of facade types (layers in the texture array).
pub const NUM_FACADE_TYPES: u32 = 5;

/// Plugin for facade texture management.
pub struct FacadeTexturesPlugin;

impl Plugin for FacadeTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_facade_textures);
    }
}

/// Resource containing the facade texture array handles.
#[derive(Resource)]
pub struct FacadeTextureArray {
    /// Albedo texture array (5 layers for 5 facade types)
    pub albedo: Handle<Image>,
    /// Normal map texture array (5 layers)
    pub normal: Handle<Image>,
}

/// Generate all facade textures and create texture arrays.
fn generate_facade_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    info!("Generating procedural facade textures...");

    // Generate individual facade textures
    let brick_albedo = generate_brick_texture();
    let brick_normal = generate_brick_normal();

    let concrete_albedo = generate_concrete_texture();
    let concrete_normal = generate_concrete_normal();

    let glass_albedo = generate_glass_texture();
    let glass_normal = generate_glass_normal();

    let metal_albedo = generate_metal_texture();
    let metal_normal = generate_metal_normal();

    let painted_albedo = generate_painted_texture();
    let painted_normal = generate_painted_normal();

    // Create texture arrays by combining layers
    let albedo_array = create_texture_array(vec![
        brick_albedo,
        concrete_albedo,
        glass_albedo,
        metal_albedo,
        painted_albedo,
    ]);

    let normal_array = create_texture_array(vec![
        brick_normal,
        concrete_normal,
        glass_normal,
        metal_normal,
        painted_normal,
    ]);

    let albedo_handle = images.add(albedo_array);
    let normal_handle = images.add(normal_array);

    commands.insert_resource(FacadeTextureArray {
        albedo: albedo_handle,
        normal: normal_handle,
    });

    info!("Facade textures generated: {}x{} per layer, {} layers",
          TEXTURE_SIZE, TEXTURE_SIZE, NUM_FACADE_TYPES);
}

/// Create a 2D texture array from individual layer images.
fn create_texture_array(layers: Vec<Image>) -> Image {
    let layer_size = (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize;
    let total_size = layer_size * layers.len();
    let mut data = Vec::with_capacity(total_size);

    for layer in &layers {
        data.extend_from_slice(&layer.data);
    }

    Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: layers.len() as u32,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Create a single-layer 2D image.
fn create_image(data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Create a normal map image (linear, not sRGB).
fn create_normal_image(data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm, // Linear for normal maps
        RenderAssetUsages::RENDER_WORLD,
    )
}

// ============================================================================
// Brick Texture Generation
// ============================================================================

fn generate_brick_texture() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let brick_width = 32;
    let brick_height = 16;
    let mortar_size = 2;

    // Base brick colors (varied reds and browns)
    let brick_colors: [(u8, u8, u8); 4] = [
        (178, 102, 76),   // Terracotta
        (156, 90, 68),    // Dark brick
        (190, 115, 85),   // Light brick
        (145, 85, 65),    // Brown brick
    ];

    let mortar_color = (210, 205, 195); // Light gray mortar

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Offset every other row for brick pattern
            let row = y / (brick_height + mortar_size);
            let offset = if row % 2 == 0 { 0 } else { brick_width / 2 };

            let local_x = (x + offset) % (brick_width + mortar_size);
            let local_y = y % (brick_height + mortar_size);

            // Check if we're in mortar
            let in_mortar = local_x >= brick_width || local_y >= brick_height;

            let (r, g, b) = if in_mortar {
                mortar_color
            } else {
                // Pick brick color based on position (deterministic variation)
                let brick_idx = ((x / brick_width) + (y / brick_height) * 7) as usize % brick_colors.len();
                let base = brick_colors[brick_idx];

                // Add subtle noise
                let noise = simple_noise(x, y, 42) as i16 - 8;
                (
                    (base.0 as i16 + noise).clamp(0, 255) as u8,
                    (base.1 as i16 + noise).clamp(0, 255) as u8,
                    (base.2 as i16 + noise).clamp(0, 255) as u8,
                )
            };

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    create_image(data)
}

fn generate_brick_normal() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let brick_width = 32;
    let brick_height = 16;
    let mortar_size = 2;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            let row = y / (brick_height + mortar_size);
            let offset = if row % 2 == 0 { 0 } else { brick_width / 2 };

            let local_x = (x + offset) % (brick_width + mortar_size);
            let local_y = y % (brick_height + mortar_size);

            let in_mortar = local_x >= brick_width || local_y >= brick_height;

            // Default normal (pointing up in tangent space)
            let mut nx: i16 = 128;
            let mut ny: i16 = 128;

            if in_mortar {
                // Mortar is recessed - edges point toward brick
                if local_x >= brick_width {
                    nx = 100; // Point left toward brick
                }
                if local_y >= brick_height {
                    ny = 100; // Point up toward brick
                }
            } else {
                // Brick surface - add subtle variation
                let noise = (simple_noise(x, y, 123) as i16 - 8) / 2;
                nx = (128 + noise).clamp(0, 255);
                ny = (128 + noise).clamp(0, 255);
            }

            data[idx] = nx as u8;
            data[idx + 1] = ny as u8;
            data[idx + 2] = 255; // Z always up
            data[idx + 3] = 255;
        }
    }

    create_normal_image(data)
}

// ============================================================================
// Concrete Texture Generation
// ============================================================================

fn generate_concrete_texture() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let base_color = (180, 175, 170); // Light gray

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Multi-frequency noise for concrete texture
            let noise1 = simple_noise(x, y, 1) as i16 - 8;
            let noise2 = (simple_noise(x / 2, y / 2, 2) as i16 - 8) / 2;
            let noise3 = (simple_noise(x / 4, y / 4, 3) as i16 - 8) / 4;
            let total_noise = noise1 + noise2 + noise3;

            // Occasional darker spots (aggregate/pitting)
            let spot = if simple_noise(x, y, 99) > 245 { -20i16 } else { 0 };

            let r = (base_color.0 as i16 + total_noise + spot).clamp(0, 255) as u8;
            let g = (base_color.1 as i16 + total_noise + spot).clamp(0, 255) as u8;
            let b = (base_color.2 as i16 + total_noise + spot).clamp(0, 255) as u8;

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    create_image(data)
}

fn generate_concrete_normal() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Subtle surface variation
            let noise = (simple_noise(x, y, 456) as i16 - 8) / 4;

            data[idx] = (128 + noise).clamp(0, 255) as u8;
            data[idx + 1] = (128 + noise).clamp(0, 255) as u8;
            data[idx + 2] = 255;
            data[idx + 3] = 255;
        }
    }

    create_normal_image(data)
}

// ============================================================================
// Glass Texture Generation
// ============================================================================

fn generate_glass_texture() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let base_color = (60, 80, 100); // Dark blue-gray glass
    let frame_color = (40, 45, 50); // Dark frame

    let panel_width = 64;
    let panel_height = 80;
    let frame_size = 3;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            let local_x = x % panel_width;
            let local_y = y % panel_height;

            // Frame around each panel
            let in_frame = local_x < frame_size || local_x >= panel_width - frame_size ||
                          local_y < frame_size || local_y >= panel_height - frame_size;

            let (r, g, b) = if in_frame {
                frame_color
            } else {
                // Glass with subtle reflection gradient
                let gradient = (local_y as f32 / panel_height as f32 * 20.0) as i16;
                (
                    (base_color.0 as i16 + gradient).clamp(0, 255) as u8,
                    (base_color.1 as i16 + gradient).clamp(0, 255) as u8,
                    (base_color.2 as i16 + gradient).clamp(0, 255) as u8,
                )
            };

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    create_image(data)
}

fn generate_glass_normal() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    // Glass is very smooth - mostly flat normal
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            data[idx] = 128;
            data[idx + 1] = 128;
            data[idx + 2] = 255;
            data[idx + 3] = 255;
        }
    }

    create_normal_image(data)
}

// ============================================================================
// Metal Texture Generation
// ============================================================================

fn generate_metal_texture() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let base_color = (100, 105, 115); // Steel gray
    let rib_spacing = 16;
    let rib_width = 4;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Horizontal corrugation pattern
            let in_rib = (y % rib_spacing) < rib_width;

            let brightness_mod = if in_rib { 20i16 } else { 0 };
            let noise = (simple_noise(x, y, 789) as i16 - 8) / 4;

            let r = (base_color.0 as i16 + brightness_mod + noise).clamp(0, 255) as u8;
            let g = (base_color.1 as i16 + brightness_mod + noise).clamp(0, 255) as u8;
            let b = (base_color.2 as i16 + brightness_mod + noise).clamp(0, 255) as u8;

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    create_image(data)
}

fn generate_metal_normal() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    let rib_spacing = 16;
    let rib_width = 4;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            let rib_pos = y % rib_spacing;

            // Normal variation for corrugated surface
            let ny = if rib_pos < rib_width / 2 {
                160 // Facing up on rib rise
            } else if rib_pos < rib_width {
                96  // Facing down on rib fall
            } else {
                128 // Flat between ribs
            };

            data[idx] = 128;
            data[idx + 1] = ny;
            data[idx + 2] = 255;
            data[idx + 3] = 255;
        }
    }

    create_normal_image(data)
}

// ============================================================================
// Painted Texture Generation
// ============================================================================

fn generate_painted_texture() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    // Cream/off-white painted surface
    let base_color = (245, 240, 230);

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Very subtle noise for paint texture
            let noise = (simple_noise(x, y, 321) as i16 - 8) / 8;

            let r = (base_color.0 as i16 + noise).clamp(0, 255) as u8;
            let g = (base_color.1 as i16 + noise).clamp(0, 255) as u8;
            let b = (base_color.2 as i16 + noise).clamp(0, 255) as u8;

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    create_image(data)
}

fn generate_painted_normal() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];

    // Painted surface is smooth
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;

            // Very subtle brush stroke variation
            let noise = (simple_noise(x, y, 654) as i16 - 8) / 8;

            data[idx] = (128 + noise).clamp(0, 255) as u8;
            data[idx + 1] = 128;
            data[idx + 2] = 255;
            data[idx + 3] = 255;
        }
    }

    create_normal_image(data)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Simple hash-based noise function (deterministic).
fn simple_noise(x: u32, y: u32, seed: u32) -> u8 {
    let mut h = x.wrapping_mul(374761393);
    h = h.wrapping_add(y.wrapping_mul(668265263));
    h = h.wrapping_add(seed);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = h ^ (h >> 16);
    (h & 0xFF) as u8
}

/// Get the texture array layer index for a facade style.
pub fn facade_to_layer(facade: FacadeStyle) -> u32 {
    match facade {
        FacadeStyle::Brick => 0,
        FacadeStyle::Concrete => 1,
        FacadeStyle::Glass => 2,
        FacadeStyle::Metal => 3,
        FacadeStyle::Painted => 4,
    }
}

/// Get facade style from layer index.
pub fn layer_to_facade(layer: u32) -> Option<FacadeStyle> {
    match layer {
        0 => Some(FacadeStyle::Brick),
        1 => Some(FacadeStyle::Concrete),
        2 => Some(FacadeStyle::Glass),
        3 => Some(FacadeStyle::Metal),
        4 => Some(FacadeStyle::Painted),
        _ => None,
    }
}

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
    /// Roughness texture array (5 layers, R8 single-channel)
    pub roughness: Handle<Image>,
    /// Metallic texture array (5 layers, R8 single-channel)
    pub metallic: Handle<Image>,
    /// Height/displacement texture array (5 layers, R8 single-channel) for POM
    pub height: Handle<Image>,
}

/// Marker resource indicating facade textures have been generated.
#[derive(Resource, Default)]
pub struct FacadeTexturesGenerated(pub bool);

/// Generate all facade textures and create texture arrays.
fn generate_facade_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    info!("Generating procedural facade textures...");

    // Generate individual facade textures with weathering applied
    let brick_albedo = generate_brick_texture_weathered();
    let brick_normal = generate_brick_normal();

    let concrete_albedo = generate_concrete_texture_weathered();
    let concrete_normal = generate_concrete_normal();

    let glass_albedo = generate_glass_texture_weathered();
    let glass_normal = generate_glass_normal();

    let metal_albedo = generate_metal_texture_weathered();
    let metal_normal = generate_metal_normal();

    let painted_albedo = generate_painted_texture_weathered();
    let painted_normal = generate_painted_normal();

    // Generate PBR maps (roughness, metallic, height)
    let brick_roughness = generate_brick_roughness();
    let concrete_roughness = generate_concrete_roughness();
    let glass_roughness = generate_glass_roughness();
    let metal_roughness = generate_metal_roughness();
    let painted_roughness = generate_painted_roughness();

    let brick_metallic = generate_brick_metallic();
    let concrete_metallic = generate_concrete_metallic();
    let glass_metallic = generate_glass_metallic();
    let metal_metallic = generate_metal_metallic();
    let painted_metallic = generate_painted_metallic();

    let brick_height = generate_brick_height();
    let concrete_height = generate_concrete_height();
    let glass_height = generate_glass_height();
    let metal_height = generate_metal_height();
    let painted_height = generate_painted_height();

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

    // Create R8 texture arrays for PBR maps
    let roughness_array = create_texture_array_r8(vec![
        brick_roughness,
        concrete_roughness,
        glass_roughness,
        metal_roughness,
        painted_roughness,
    ]);

    let metallic_array = create_texture_array_r8(vec![
        brick_metallic,
        concrete_metallic,
        glass_metallic,
        metal_metallic,
        painted_metallic,
    ]);

    let height_array = create_texture_array_r8(vec![
        brick_height,
        concrete_height,
        glass_height,
        metal_height,
        painted_height,
    ]);

    let albedo_handle = images.add(albedo_array);
    let normal_handle = images.add(normal_array);
    let roughness_handle = images.add(roughness_array);
    let metallic_handle = images.add(metallic_array);
    let height_handle = images.add(height_array);

    commands.insert_resource(FacadeTextureArray {
        albedo: albedo_handle,
        normal: normal_handle,
        roughness: roughness_handle,
        metallic: metallic_handle,
        height: height_handle,
    });

    // Mark textures as generated for dependent systems
    commands.insert_resource(FacadeTexturesGenerated(true));

    info!("Facade textures generated: {}x{} per layer, {} layers (albedo, normal, roughness, metallic, height)",
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

/// Create a single-channel R8 image (for roughness, metallic, height maps).
fn create_r8_image(data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Create a 2D texture array from single-channel R8 layer images.
fn create_texture_array_r8(layers: Vec<Image>) -> Image {
    let layer_size = (TEXTURE_SIZE * TEXTURE_SIZE) as usize; // 1 byte per pixel
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
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

// ============================================================================
// Brick Texture Generation
// ============================================================================

fn generate_brick_texture_weathered() -> Image {
    let mut img = generate_brick_texture();
    apply_weathering(&mut img.data, TEXTURE_SIZE, TEXTURE_SIZE,
                     facade_weathering_intensity(FacadeStyle::Brick), 1001);
    img
}

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

fn generate_concrete_texture_weathered() -> Image {
    let mut img = generate_concrete_texture();
    apply_weathering(&mut img.data, TEXTURE_SIZE, TEXTURE_SIZE,
                     facade_weathering_intensity(FacadeStyle::Concrete), 2002);
    img
}

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

fn generate_glass_texture_weathered() -> Image {
    let mut img = generate_glass_texture();
    apply_weathering(&mut img.data, TEXTURE_SIZE, TEXTURE_SIZE,
                     facade_weathering_intensity(FacadeStyle::Glass), 3003);
    img
}

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

fn generate_metal_texture_weathered() -> Image {
    let mut img = generate_metal_texture();
    apply_weathering(&mut img.data, TEXTURE_SIZE, TEXTURE_SIZE,
                     facade_weathering_intensity(FacadeStyle::Metal), 4004);
    img
}

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

fn generate_painted_texture_weathered() -> Image {
    let mut img = generate_painted_texture();
    apply_weathering(&mut img.data, TEXTURE_SIZE, TEXTURE_SIZE,
                     facade_weathering_intensity(FacadeStyle::Painted), 5005);
    img
}

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
// Roughness Map Generation (R8 single-channel, 0=smooth, 255=rough)
// ============================================================================

fn generate_brick_roughness() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let brick_width = 32;
    let brick_height = 16;
    let mortar_size = 2;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let row = y / (brick_height + mortar_size);
            let offset = if row % 2 == 0 { 0 } else { brick_width / 2 };

            let local_x = (x + offset) % (brick_width + mortar_size);
            let local_y = y % (brick_height + mortar_size);

            let in_mortar = local_x >= brick_width || local_y >= brick_height;

            let base_roughness = if in_mortar {
                200 // Mortar is slightly smoother than brick
            } else {
                230 // Brick is very rough
            };

            // Add noise variation
            let noise = (simple_noise(x, y, 1111) as i16 - 128) / 8;
            data[idx] = (base_roughness as i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

fn generate_concrete_roughness() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            // Base roughness with multi-frequency noise
            let noise1 = simple_noise(x, y, 2222) as i16 - 128;
            let noise2 = (simple_noise(x / 2, y / 2, 2223) as i16 - 128) / 2;

            // Occasional smooth patches (polished spots)
            let spot = if simple_noise(x / 8, y / 8, 2224) > 240 { -30i16 } else { 0 };

            let roughness = (200i16 + noise1 / 4 + noise2 / 4 + spot).clamp(0, 255) as u8;
            data[idx] = roughness;
        }
    }

    create_r8_image(data)
}

fn generate_glass_roughness() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let panel_width = 64;
    let panel_height = 80;
    let frame_size = 3;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let local_x = x % panel_width;
            let local_y = y % panel_height;

            let in_frame = local_x < frame_size || local_x >= panel_width - frame_size ||
                          local_y < frame_size || local_y >= panel_height - frame_size;

            // Glass is very smooth, frame is rougher
            let roughness = if in_frame {
                150 // Metal/plastic frame
            } else {
                25  // Very smooth glass
            };

            data[idx] = roughness;
        }
    }

    create_r8_image(data)
}

fn generate_metal_roughness() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let rib_spacing = 16;
    let rib_width = 4;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let rib_pos = y % rib_spacing;
            let on_rib = rib_pos < rib_width;

            // Ribs are slightly smoother (more worn/polished)
            let base_roughness = if on_rib { 80 } else { 110 };

            // Add subtle noise
            let noise = (simple_noise(x, y, 4444) as i16 - 128) / 16;
            data[idx] = (base_roughness as i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

fn generate_painted_roughness() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            // Painted surface with subtle brush stroke variation
            let noise = (simple_noise(x, y, 5555) as i16 - 128) / 8;
            data[idx] = (180i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

// ============================================================================
// Metallic Map Generation (R8 single-channel, 0=dielectric, 255=metal)
// ============================================================================

fn generate_brick_metallic() -> Image {
    // Brick is entirely non-metallic
    let data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];
    create_r8_image(data)
}

fn generate_concrete_metallic() -> Image {
    // Concrete is entirely non-metallic
    let data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];
    create_r8_image(data)
}

fn generate_glass_metallic() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let panel_width = 64;
    let panel_height = 80;
    let frame_size = 3;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let local_x = x % panel_width;
            let local_y = y % panel_height;

            let in_frame = local_x < frame_size || local_x >= panel_width - frame_size ||
                          local_y < frame_size || local_y >= panel_height - frame_size;

            // Frame is slightly metallic (aluminum), glass is not
            data[idx] = if in_frame { 180 } else { 0 };
        }
    }

    create_r8_image(data)
}

fn generate_metal_metallic() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            // High metallic value with slight variation
            let noise = (simple_noise(x, y, 6666) as i16 - 128) / 16;
            data[idx] = (200i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

fn generate_painted_metallic() -> Image {
    // Painted surface is non-metallic
    let data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];
    create_r8_image(data)
}

// ============================================================================
// Height Map Generation (R8 single-channel, 128=base, <128=recessed, >128=raised)
// Used for Parallax Occlusion Mapping
// ============================================================================

fn generate_brick_height() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let brick_width = 32;
    let brick_height = 16;
    let mortar_size = 2;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let row = y / (brick_height + mortar_size);
            let offset = if row % 2 == 0 { 0 } else { brick_width / 2 };

            let local_x = (x + offset) % (brick_width + mortar_size);
            let local_y = y % (brick_height + mortar_size);

            let in_mortar = local_x >= brick_width || local_y >= brick_height;

            // Mortar is recessed, bricks are raised
            let base_height = if in_mortar {
                70  // Mortar is recessed
            } else {
                170 // Brick surface is raised
            };

            // Add subtle surface variation to bricks
            let noise = if in_mortar {
                0
            } else {
                (simple_noise(x, y, 7777) as i16 - 128) / 16
            };

            data[idx] = (base_height as i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

fn generate_concrete_height() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            // Subtle surface variation
            let noise1 = (simple_noise(x, y, 8888) as i16 - 128) / 8;
            let noise2 = (simple_noise(x / 2, y / 2, 8889) as i16 - 128) / 16;

            // Occasional pits
            let pit = if simple_noise(x, y, 8890) > 250 { -20i16 } else { 0 };

            data[idx] = (128i16 + noise1 + noise2 + pit).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

fn generate_glass_height() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let panel_width = 64;
    let panel_height = 80;
    let frame_size = 3;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let local_x = x % panel_width;
            let local_y = y % panel_height;

            let in_frame = local_x < frame_size || local_x >= panel_width - frame_size ||
                          local_y < frame_size || local_y >= panel_height - frame_size;

            // Glass is flat, frame is slightly raised
            data[idx] = if in_frame { 150 } else { 128 };
        }
    }

    create_r8_image(data)
}

fn generate_metal_height() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    let rib_spacing = 16;
    let rib_width = 4;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            let rib_pos = y % rib_spacing;

            // Create corrugation profile
            let height = if rib_pos < rib_width / 2 {
                // Rising edge of rib
                let t = rib_pos as f32 / (rib_width as f32 / 2.0);
                (128.0 + 50.0 * t) as u8
            } else if rib_pos < rib_width {
                // Falling edge of rib
                let t = (rib_pos - rib_width / 2) as f32 / (rib_width as f32 / 2.0);
                (178.0 - 50.0 * t) as u8
            } else {
                // Valley between ribs
                100
            };

            data[idx] = height;
        }
    }

    create_r8_image(data)
}

fn generate_painted_height() -> Image {
    let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE) as usize];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let idx = (y * TEXTURE_SIZE + x) as usize;

            // Very subtle brush stroke texture
            let noise = (simple_noise(x, y, 9999) as i16 - 128) / 32;
            data[idx] = (128i16 + noise).clamp(0, 255) as u8;
        }
    }

    create_r8_image(data)
}

// ============================================================================
// Weathering Effects
// ============================================================================

/// Apply weathering effects to a texture (water stains, dirt, wear).
fn apply_weathering(data: &mut [u8], width: u32, height: u32, intensity: f32, seed: u32) {
    let intensity = intensity.clamp(0.0, 1.0);
    if intensity < 0.01 {
        return;
    }

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;

            let mut darken = 0.0f32;

            // 1. Vertical water stain streaks from top
            // Create columnar noise that flows downward
            let column_noise = simple_noise(x / 8, 0, seed.wrapping_add(100)) as f32 / 255.0;
            if column_noise > 0.7 {
                // This column has a water stain
                let stain_strength = (column_noise - 0.7) / 0.3; // 0-1
                let y_factor = (y as f32 / height as f32).powf(0.5); // Stronger at top, fades down
                let streak_noise = simple_noise(x, y / 4, seed.wrapping_add(200)) as f32 / 255.0;
                darken += stain_strength * y_factor * (0.5 + streak_noise * 0.5) * 0.15;
            }

            // 2. Dirt accumulation at bottom (gradient)
            let bottom_factor = 1.0 - (y as f32 / height as f32);
            if bottom_factor < 0.15 {
                let dirt_strength = 1.0 - (bottom_factor / 0.15);
                let dirt_noise = simple_noise(x, y, seed.wrapping_add(300)) as f32 / 255.0;
                darken += dirt_strength * (0.6 + dirt_noise * 0.4) * 0.12;
            }

            // 3. Random stains/spots
            let spot_noise = simple_noise(x / 16, y / 16, seed.wrapping_add(400)) as f32 / 255.0;
            if spot_noise > 0.85 {
                let fine_noise = simple_noise(x, y, seed.wrapping_add(500)) as f32 / 255.0;
                darken += (spot_noise - 0.85) / 0.15 * fine_noise * 0.1;
            }

            // 4. Edge wear (corners of texture - simulates building corners)
            let edge_x = (x as f32 / width as f32 - 0.5).abs() * 2.0;
            let edge_y = (y as f32 / height as f32 - 0.5).abs() * 2.0;
            let edge_factor = (edge_x * edge_y).powf(2.0);
            if edge_factor > 0.5 {
                darken += (edge_factor - 0.5) * 0.08;
            }

            // Apply darkening scaled by intensity
            let total_darken = (darken * intensity).clamp(0.0, 0.3);
            let multiplier = 1.0 - total_darken;

            // Also shift slightly toward brown/gray for weathered look
            let brown_shift = total_darken * 0.3;

            let r = data[idx] as f32;
            let g = data[idx + 1] as f32;
            let b = data[idx + 2] as f32;

            data[idx] = ((r * multiplier) + (brown_shift * 20.0)).clamp(0.0, 255.0) as u8;
            data[idx + 1] = ((g * multiplier) - (brown_shift * 10.0)).clamp(0.0, 255.0) as u8;
            data[idx + 2] = ((b * multiplier) - (brown_shift * 15.0)).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Get default weathering intensity for a facade style.
fn facade_weathering_intensity(facade: FacadeStyle) -> f32 {
    match facade {
        FacadeStyle::Brick => 0.6,      // Brick shows weathering well
        FacadeStyle::Concrete => 0.7,   // Concrete gets dirty easily
        FacadeStyle::Glass => 0.1,      // Glass is cleaned regularly
        FacadeStyle::Metal => 0.5,      // Metal gets some weathering
        FacadeStyle::Painted => 0.4,    // Painted surfaces weather moderately
    }
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

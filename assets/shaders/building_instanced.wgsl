// Extended GPU Instancing shader for city buildings with PBR+ and POM
// Supports full 4x4 transform matrix, material parameters, PBR lighting, POM, and texture arrays
//
// Instance data layout (128 bytes):
// - transform (mat4x4): locations 5-8
// - color (vec4): location 9
// - material_params (vec4): location 10 (roughness, metallic, emissive, facade_type)
// - bounds (vec4): location 11 (half_extents xyz, mesh_type w)
// - extra (vec4): location 12 (archetype, lot_index, flags, reserved)
//
// Facade types (texture array layers):
// 0 = Brick, 1 = Concrete, 2 = Glass, 3 = Metal, 4 = Painted

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    pbr_types,
    pbr_functions,
}

// Material uniforms (must match BuildingMaterialUniforms in Rust)
struct BuildingMaterial {
    base_color: vec4<f32>,
    time_of_day: f32,
    use_textures: f32,
    pom_scale: f32,      // Parallax Occlusion Mapping depth scale (0.02-0.1)
    pom_layers: f32,     // Number of POM ray march steps (16-64)
};

@group(2) @binding(0) var<uniform> material: BuildingMaterial;

// Texture arrays for PBR+ materials
@group(2) @binding(1) var facade_albedo: texture_2d_array<f32>;
@group(2) @binding(2) var facade_normal: texture_2d_array<f32>;
@group(2) @binding(3) var facade_roughness: texture_2d_array<f32>;
@group(2) @binding(4) var facade_metallic: texture_2d_array<f32>;
@group(2) @binding(5) var facade_height: texture_2d_array<f32>;
@group(2) @binding(6) var facade_sampler: sampler;

// Constants
const PI: f32 = 3.14159265359;
const POM_MIN_LAYERS: f32 = 8.0;
const POM_MAX_LAYERS: f32 = 64.0;
const POM_DISTANCE_CUTOFF: f32 = 150.0;  // Disable POM beyond this distance

// Per-instance data from vertex buffer
struct BuildingInstance {
    @location(5) transform_col0: vec4<f32>,
    @location(6) transform_col1: vec4<f32>,
    @location(7) transform_col2: vec4<f32>,
    @location(8) transform_col3: vec4<f32>,
    @location(9) color: vec4<f32>,
    @location(10) material_params: vec4<f32>,  // roughness, metallic, emissive, facade_type
    @location(11) bounds: vec4<f32>,            // half_extents xyz, mesh_type w
    @location(12) extra: vec4<f32>,             // archetype, lot_index, flags, reserved
};

// Vertex input from mesh
struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// Custom vertex output with additional instance data
struct BuildingVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) material_params: vec4<f32>,
    @location(5) extra: vec4<f32>,
    @location(6) tangent: vec3<f32>,
    @location(7) bitangent: vec3<f32>,
};

@vertex
fn vertex(vertex: VertexInput, instance: BuildingInstance) -> BuildingVertexOutput {
    var out: BuildingVertexOutput;

    // Reconstruct 4x4 transform matrix from columns
    let model_matrix = mat4x4<f32>(
        instance.transform_col0,
        instance.transform_col1,
        instance.transform_col2,
        instance.transform_col3
    );

    // Transform position
    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_position;
    out.clip_position = view.clip_from_world * world_position;

    // Transform normal (use inverse transpose of 3x3 for correct normal transformation)
    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz
    );
    out.world_normal = normalize(normal_matrix * vertex.normal);

    // Calculate tangent and bitangent for normal mapping and POM
    // Use world up and normal to create tangent frame
    let world_up = vec3<f32>(0.0, 1.0, 0.0);
    var tangent = normalize(cross(world_up, out.world_normal));
    if (length(tangent) < 0.01) {
        // Normal is parallel to up, use different reference
        tangent = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), out.world_normal));
    }
    let bitangent = normalize(cross(out.world_normal, tangent));

    out.tangent = tangent;
    out.bitangent = bitangent;

    // Scale UVs based on facade type for proper texture tiling
    let facade_type = u32(instance.material_params.w);
    let uv_scale = get_facade_uv_scale(facade_type);
    out.uv = vertex.uv * uv_scale;

    // Pass instance data to fragment shader
    out.color = instance.color;
    out.material_params = instance.material_params;
    out.extra = instance.extra;

    return out;
}

// ============================================================================
// Parallax Occlusion Mapping (POM)
// ============================================================================

// Get POM depth scale per facade type (different materials have different depth)
fn get_facade_pom_scale(facade_type: u32) -> f32 {
    switch facade_type {
        case 0u: { return 0.06; }  // Brick - deep mortar grooves
        case 1u: { return 0.02; }  // Concrete - subtle surface noise
        case 2u: { return 0.01; }  // Glass - nearly flat
        case 3u: { return 0.04; }  // Metal - corrugation ridges
        case 4u: { return 0.01; }  // Painted - minimal texture
        default: { return 0.02; }
    }
}

// Ray march through height field to find intersection point
fn parallax_occlusion_mapping(
    uv: vec2<f32>,
    view_dir_tangent: vec3<f32>,
    facade_type: i32,
    height_scale: f32,
    num_layers: f32,
) -> vec2<f32> {
    // Calculate step size and direction in UV space
    let layer_depth = 1.0 / num_layers;

    // The direction to step in UV space (scaled by view angle for proper depth)
    let p = view_dir_tangent.xy / max(view_dir_tangent.z, 0.001) * height_scale;
    let delta_uv = p / num_layers;

    var current_layer_depth: f32 = 0.0;
    var current_uv = uv;

    // Sample initial height (height map: 128 = base, <128 = recessed, >128 = raised)
    var current_height = textureSample(facade_height, facade_sampler, current_uv, facade_type).r;
    // Convert from 0-1 to -0.5 to 0.5 (128/255 = ~0.5 is the base)
    current_height = current_height - 0.5;

    // Ray march through height field
    var prev_uv = current_uv;
    var prev_layer_depth = current_layer_depth;
    var prev_height = current_height;

    // Step through layers until we find intersection
    for (var i: i32 = 0; i < i32(num_layers); i = i + 1) {
        if (current_layer_depth > current_height) {
            break;
        }

        prev_uv = current_uv;
        prev_layer_depth = current_layer_depth;
        prev_height = current_height;

        current_uv = current_uv - delta_uv;
        current_layer_depth = current_layer_depth + layer_depth;

        let height_sample = textureSample(facade_height, facade_sampler, current_uv, facade_type).r;
        current_height = height_sample - 0.5;
    }

    // Linear interpolation for smoother result
    let after_depth = current_height - current_layer_depth;
    let before_depth = prev_height - prev_layer_depth;
    let weight = after_depth / (after_depth - before_depth);

    return mix(current_uv, prev_uv, weight);
}

// Calculate self-shadow from POM (soft shadow in recessed areas)
fn pom_self_shadow(
    uv: vec2<f32>,
    light_dir_tangent: vec3<f32>,
    facade_type: i32,
    initial_height: f32,
    height_scale: f32,
) -> f32 {
    // Simplified shadow check - trace toward light
    let num_shadow_layers: f32 = 8.0;
    let layer_depth = 1.0 / num_shadow_layers;

    let p = light_dir_tangent.xy / max(light_dir_tangent.z, 0.001) * height_scale;
    let delta_uv = p / num_shadow_layers;

    var shadow: f32 = 1.0;
    var current_layer_depth = initial_height;
    var current_uv = uv;

    for (var i: i32 = 0; i < i32(num_shadow_layers); i = i + 1) {
        current_uv = current_uv + delta_uv;
        current_layer_depth = current_layer_depth - layer_depth;

        let height_sample = textureSample(facade_height, facade_sampler, current_uv, facade_type).r - 0.5;

        if (height_sample > current_layer_depth) {
            // In shadow
            let shadow_factor = (height_sample - current_layer_depth) * 4.0;
            shadow = min(shadow, 1.0 - clamp(shadow_factor, 0.0, 1.0));
        }
    }

    return shadow;
}

// ============================================================================
// PBR Lighting (Cook-Torrance BRDF)
// ============================================================================

// Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

// GGX/Trowbridge-Reitz normal distribution function
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// Smith's Schlick-GGX geometry function (single direction)
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

// Smith's geometry function (combined for view and light)
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx1 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx2 = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx1 * ggx2;
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fragment(in: BuildingVertexOutput) -> @location(0) vec4<f32> {
    // Check visibility flag
    let flags = u32(in.extra.z);
    if ((flags & 1u) == 0u) {
        discard;
    }

    // Extract instance material parameters
    let instance_roughness = in.material_params.x;
    let instance_metallic = in.material_params.y;
    let emissive_intensity = in.material_params.z;
    let facade_type = u32(in.material_params.w);

    // Calculate view direction
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let view_distance = length(view.world_position.xyz - in.world_position.xyz);

    // Build TBN matrix for tangent space transformations
    let tbn = mat3x3<f32>(in.tangent, in.bitangent, in.world_normal);
    let tbn_inverse = transpose(tbn);  // For world-to-tangent

    // Transform view direction to tangent space for POM
    let view_dir_tangent = normalize(tbn_inverse * view_dir);

    // Default values
    var base_color = in.color;
    var normal = in.world_normal;
    var roughness = instance_roughness;
    var metallic = instance_metallic;
    var uv = in.uv;

    if (material.use_textures > 0.5) {
        // Apply POM if within distance threshold
        let pom_enabled = view_distance < POM_DISTANCE_CUTOFF;

        if (pom_enabled) {
            // LOD-based layer count: more layers when close, fewer when far
            let distance_factor = clamp(view_distance / POM_DISTANCE_CUTOFF, 0.0, 1.0);
            let num_layers = mix(material.pom_layers, POM_MIN_LAYERS, distance_factor);

            // Get per-facade POM scale
            let pom_scale = get_facade_pom_scale(facade_type) * material.pom_scale / 0.05;

            // Apply parallax occlusion mapping
            uv = parallax_occlusion_mapping(
                in.uv,
                view_dir_tangent,
                i32(facade_type),
                pom_scale,
                num_layers
            );
        }

        // Sample albedo from texture array with POM-adjusted UVs
        let albedo_sample = textureSample(facade_albedo, facade_sampler, uv, i32(facade_type));

        // Blend texture with instance color for variation
        base_color = vec4<f32>(
            albedo_sample.rgb * in.color.rgb,
            in.color.a
        );

        // Sample and apply normal map
        let normal_sample = textureSample(facade_normal, facade_sampler, uv, i32(facade_type));
        let tangent_normal = normal_sample.rgb * 2.0 - 1.0;

        // Get normal strength based on facade type
        let normal_strength = get_facade_normal_strength(facade_type);
        let adjusted_tangent_normal = vec3<f32>(
            tangent_normal.x * normal_strength,
            tangent_normal.y * normal_strength,
            tangent_normal.z
        );

        // Transform from tangent space to world space
        normal = normalize(tbn * adjusted_tangent_normal);

        // Sample roughness and metallic from PBR texture arrays
        let roughness_sample = textureSample(facade_roughness, facade_sampler, uv, i32(facade_type)).r;
        let metallic_sample = textureSample(facade_metallic, facade_sampler, uv, i32(facade_type)).r;

        // Blend sampled values with instance values (instance can modify)
        roughness = mix(instance_roughness, roughness_sample, 0.8);
        metallic = mix(instance_metallic, metallic_sample, 0.8);
    }

    // Sun direction (TODO: pass as uniform for day/night cycle)
    let sun_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let sun_color = vec3<f32>(1.0, 0.95, 0.9);
    let sun_intensity: f32 = 2.0;

    // PBR lighting calculations
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, sun_dir), 0.0);
    let half_dir = normalize(sun_dir + view_dir);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);

    // Base reflectivity (F0) - dielectrics ~0.04, metals use albedo
    let f0 = mix(vec3<f32>(0.04), base_color.rgb, metallic);

    // Cook-Torrance BRDF components
    let D = distribution_ggx(n_dot_h, roughness);
    let G = geometry_smith(n_dot_v, n_dot_l, roughness);
    let F = fresnel_schlick(h_dot_v, f0);

    // Specular term
    let numerator = D * G * F;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001;
    let specular = numerator / denominator;

    // Energy conservation: kS + kD = 1
    let kS = F;
    let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);

    // Diffuse term (Lambertian)
    let diffuse = kD * base_color.rgb / PI;

    // Direct lighting
    let direct_light = (diffuse + specular) * sun_color * sun_intensity * n_dot_l;

    // Ambient with hemisphere lighting approximation
    let sky_color = vec3<f32>(0.5, 0.6, 0.8);
    let ground_color = vec3<f32>(0.2, 0.2, 0.15);
    let hemisphere_factor = normal.y * 0.5 + 0.5;
    let ambient_color = mix(ground_color, sky_color, hemisphere_factor);

    // Ambient uses Fresnel for environment reflection approximation
    let ambient_fresnel = fresnel_schlick(n_dot_v, f0);
    let ambient_specular = ambient_fresnel * ambient_color * 0.2;
    let ambient_diffuse = (1.0 - ambient_fresnel) * (1.0 - metallic) * base_color.rgb * ambient_color * 0.15;
    let ambient = ambient_diffuse + ambient_specular;

    // POM self-shadow (only when textures and POM are enabled)
    var shadow: f32 = 1.0;
    if (material.use_textures > 0.5 && view_distance < POM_DISTANCE_CUTOFF) {
        let light_dir_tangent = normalize(tbn_inverse * sun_dir);
        let height_sample = textureSample(facade_height, facade_sampler, uv, i32(facade_type)).r - 0.5;
        let pom_scale = get_facade_pom_scale(facade_type) * material.pom_scale / 0.05;
        shadow = pom_self_shadow(uv, light_dir_tangent, i32(facade_type), height_sample, pom_scale);
    }

    // Combine lighting
    var final_color = ambient + direct_light * shadow;

    // Add emissive for night lighting (windows, signs)
    if (emissive_intensity > 0.0) {
        final_color = final_color + base_color.rgb * emissive_intensity;
    }

    return vec4<f32>(final_color, base_color.a);
}

// ============================================================================
// Helper Functions
// ============================================================================

// Get UV scale for each facade type (tiling)
fn get_facade_uv_scale(facade_type: u32) -> vec2<f32> {
    switch facade_type {
        case 0u: { return vec2<f32>(4.0, 4.0); }  // Brick - small tiles
        case 1u: { return vec2<f32>(2.0, 2.0); }  // Concrete - larger panels
        case 2u: { return vec2<f32>(1.0, 2.0); }  // Glass - vertical windows
        case 3u: { return vec2<f32>(3.0, 1.0); }  // Metal - horizontal panels
        case 4u: { return vec2<f32>(1.0, 1.0); }  // Painted - uniform
        default: { return vec2<f32>(1.0, 1.0); }
    }
}

// Get normal map strength for each facade type
fn get_facade_normal_strength(facade_type: u32) -> f32 {
    switch facade_type {
        case 0u: { return 1.0; }   // Brick - strong normal detail
        case 1u: { return 0.5; }   // Concrete - medium detail
        case 2u: { return 0.2; }   // Glass - subtle reflection bumps
        case 3u: { return 0.8; }   // Metal - corrugated panels
        case 4u: { return 0.3; }   // Painted - subtle texture
        default: { return 0.5; }
    }
}

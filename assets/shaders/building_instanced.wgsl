// Extended GPU Instancing shader for city buildings
// Supports full 4x4 transform matrix, material parameters, PBR lighting, and texture arrays
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

// Material uniforms
struct BuildingMaterial {
    base_color: vec4<f32>,
    time_of_day: f32,
    use_textures: f32,  // 1.0 to use texture arrays, 0.0 for solid colors
    _padding: vec2<f32>,
};

@group(2) @binding(0) var<uniform> material: BuildingMaterial;

// Texture arrays (optional - may not be bound)
@group(2) @binding(1) var facade_albedo: texture_2d_array<f32>;
@group(2) @binding(2) var facade_normal: texture_2d_array<f32>;
@group(2) @binding(3) var facade_sampler: sampler;

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

    // Calculate tangent and bitangent for normal mapping
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

@fragment
fn fragment(in: BuildingVertexOutput) -> @location(0) vec4<f32> {
    // Extract material parameters
    let roughness = in.material_params.x;
    let metallic = in.material_params.y;
    let emissive_intensity = in.material_params.z;
    let facade_type = u32(in.material_params.w);

    // Check visibility flag
    let flags = u32(in.extra.z);
    if ((flags & 1u) == 0u) {
        discard;
    }

    // Sample textures if enabled
    var base_color = in.color;
    var normal = in.world_normal;

    if (material.use_textures > 0.5) {
        // Sample albedo from texture array
        let albedo_sample = textureSample(facade_albedo, facade_sampler, in.uv, i32(facade_type));

        // Blend texture with instance color for variation
        base_color = vec4<f32>(
            albedo_sample.rgb * in.color.rgb,
            in.color.a
        );

        // Sample and apply normal map
        let normal_sample = textureSample(facade_normal, facade_sampler, in.uv, i32(facade_type));
        let tangent_normal = normal_sample.rgb * 2.0 - 1.0;

        // Get normal strength based on facade type
        let normal_strength = get_facade_normal_strength(facade_type);
        let adjusted_tangent_normal = vec3<f32>(
            tangent_normal.x * normal_strength,
            tangent_normal.y * normal_strength,
            tangent_normal.z
        );

        // Transform from tangent space to world space
        let tbn = mat3x3<f32>(in.tangent, in.bitangent, in.world_normal);
        normal = normalize(tbn * adjusted_tangent_normal);
    }

    // Simple PBR-like lighting
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);

    // Sun direction (could be passed as uniform for day/night cycle)
    let sun_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));

    // Ambient with hemisphere lighting approximation
    let sky_color = vec3<f32>(0.5, 0.6, 0.8);
    let ground_color = vec3<f32>(0.2, 0.2, 0.15);
    let hemisphere_factor = normal.y * 0.5 + 0.5;
    let ambient_color = mix(ground_color, sky_color, hemisphere_factor);
    let ambient = 0.2 * ambient_color * base_color.rgb;

    // Diffuse
    let n_dot_l = max(dot(normal, sun_dir), 0.0);
    let diffuse = n_dot_l * base_color.rgb;

    // Specular (Blinn-Phong approximation)
    let half_dir = normalize(sun_dir + view_dir);
    let n_dot_h = max(dot(normal, half_dir), 0.0);

    // Roughness affects specular power
    let specular_power = mix(128.0, 8.0, roughness);
    let specular_strength = mix(0.5, 0.1, roughness) * metallic;
    let specular = specular_strength * pow(n_dot_h, specular_power) * vec3<f32>(1.0, 1.0, 1.0);

    // Fresnel effect for metallic surfaces
    let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0) * metallic * 0.3;
    let fresnel_color = mix(base_color.rgb, sky_color, fresnel);

    // Combine lighting
    var final_color = ambient + diffuse * 0.7 + specular + fresnel_color * fresnel;

    // Add emissive for night lighting (windows, signs)
    if (emissive_intensity > 0.0) {
        final_color += base_color.rgb * emissive_intensity;
    }

    return vec4<f32>(final_color, base_color.a);
}

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

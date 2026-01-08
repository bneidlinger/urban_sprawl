// GPU Instancing shader for city windows
// Renders hundreds of thousands of window quads in a single draw call
//
// Instance data layout (64 bytes):
// - position_occupied (vec4): location 5 (xyz = position, w = occupied flag)
// - size_normal (vec4): location 6 (xy = size, zw = facing normal XZ)
// - color (vec4): location 7 (night light color RGBA)
// - material_params (vec4): location 8 (intensity, facade_type, metallic, roughness)

#import bevy_pbr::{
    mesh_view_bindings::view,
}

// Material uniforms
struct WindowMaterial {
    base_color: vec4<f32>,
    time_of_day: f32,
    night_factor: f32,
    _padding: vec2<f32>,
};

@group(2) @binding(0) var<uniform> material: WindowMaterial;

// Per-instance data from vertex buffer
struct WindowInstance {
    @location(5) position_occupied: vec4<f32>,  // xyz = position, w = occupied
    @location(6) size_normal: vec4<f32>,        // xy = size, zw = normal XZ
    @location(7) color: vec4<f32>,              // night light color RGBA
    @location(8) material_params: vec4<f32>,    // intensity, facade_type, metallic, roughness
};

// Vertex input from quad mesh
struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// Custom vertex output
struct WindowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) material_params: vec4<f32>,
    @location(5) occupied: f32,
};

@vertex
fn vertex(vertex: VertexInput, instance: WindowInstance) -> WindowVertexOutput {
    var out: WindowVertexOutput;

    // Extract instance data
    let window_pos = instance.position_occupied.xyz;
    let occupied = instance.position_occupied.w;
    let window_size = instance.size_normal.xy;
    let normal_xz = instance.size_normal.zw;

    // Build rotation matrix from normal direction
    // The normal points outward from the building face
    let normal_3d = normalize(vec3<f32>(normal_xz.x, 0.0, normal_xz.y));

    // Calculate right vector (perpendicular to normal in XZ plane)
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, normal_3d));

    // Transform local quad position to world space
    // Input quad is assumed to be a unit quad in XY plane centered at origin
    let local_pos = vertex.position;

    // Scale by window size and orient to face the normal direction
    let world_offset = right * local_pos.x * window_size.x +
                       up * local_pos.y * window_size.y +
                       normal_3d * local_pos.z * 0.08; // Slight depth

    let world_position = vec4<f32>(window_pos + world_offset, 1.0);

    out.world_position = world_position;
    out.clip_position = view.clip_from_world * world_position;

    // Transform normal
    out.world_normal = normal_3d;

    // Pass through UV coordinates
    out.uv = vertex.uv;

    // Pass instance data to fragment shader
    out.color = instance.color;
    out.material_params = instance.material_params;
    out.occupied = occupied;

    return out;
}

@fragment
fn fragment(in: WindowVertexOutput) -> @location(0) vec4<f32> {
    // Extract material parameters
    let intensity = in.material_params.x;
    let facade_type = u32(in.material_params.y);
    let metallic = in.material_params.z;
    let roughness = in.material_params.w;

    // Night factor from uniform (0 = day, 1 = night)
    let night_factor = material.night_factor;

    // Base glass color from material uniform
    var glass_color = material.base_color;

    // Adjust glass color based on facade type
    glass_color = adjust_glass_for_facade(glass_color, facade_type);

    // View direction for reflections
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);

    // Simple Fresnel effect for glass
    let n_dot_v = max(dot(in.world_normal, view_dir), 0.0);
    let fresnel = pow(1.0 - n_dot_v, 3.0) * metallic;

    // Sky reflection color (simplified)
    let sky_color = vec3<f32>(0.5, 0.6, 0.8);
    let reflection = sky_color * fresnel;

    // Base shading
    let sun_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let n_dot_l = max(dot(in.world_normal, sun_dir), 0.0);

    // Ambient + diffuse for glass
    let ambient = 0.2 * glass_color.rgb;
    let diffuse = 0.3 * n_dot_l * glass_color.rgb;

    // Combine base glass appearance
    var final_color = ambient + diffuse + reflection;

    // Apply night lighting for occupied windows
    if (in.occupied > 0.5 && night_factor > 0.0) {
        // Warm interior light shows through
        let interior_color = in.color.rgb * intensity * night_factor;

        // Blend interior light with glass (interior shows through more at night)
        let interior_blend = night_factor * 0.8;
        final_color = mix(final_color, interior_color, interior_blend);

        // Add emissive glow
        final_color += in.color.rgb * intensity * night_factor * 0.3;
    }

    // Glass alpha - more transparent at night for occupied windows
    var alpha = glass_color.a;
    if (in.occupied > 0.5 && night_factor > 0.0) {
        // Occupied windows are more opaque at night (lit from inside)
        alpha = mix(alpha, 0.9, night_factor * 0.5);
    } else {
        // Add slight Fresnel to alpha for reflective edges
        alpha = mix(alpha, 0.95, fresnel * 0.3);
    }

    return vec4<f32>(final_color, alpha);
}

// Adjust glass color based on facade style
fn adjust_glass_for_facade(base_color: vec4<f32>, facade_type: u32) -> vec4<f32> {
    switch facade_type {
        // Glass facades - blue-green tinted glass
        case 0u: {
            return vec4<f32>(0.3, 0.5, 0.7, 0.6);
        }
        // Brick facades - darker, older style glass
        case 1u: {
            return vec4<f32>(0.12, 0.12, 0.18, 0.75);
        }
        // Concrete facades - neutral gray glass
        case 2u: {
            return vec4<f32>(0.25, 0.28, 0.32, 0.65);
        }
        // Metal facades - tinted industrial glass
        case 3u: {
            return vec4<f32>(0.18, 0.2, 0.22, 0.6);
        }
        // Painted facades - clear residential glass
        case 4u: {
            return vec4<f32>(0.1, 0.1, 0.15, 0.75);
        }
        default: {
            return base_color;
        }
    }
}

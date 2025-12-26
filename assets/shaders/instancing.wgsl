// GPU Instancing shader for city buildings
// Supports per-instance position, scale, and color

#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::forward_io

struct InstanceInput {
    @location(5) i_pos_scale: vec4<f32>,   // xyz = position, w = uniform scale
    @location(6) i_color: vec4<f32>,        // rgba color
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec4<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vertex(
    vertex: forward_io::Vertex,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Apply instance transform
    let scale = instance.i_pos_scale.w;
    let world_pos = vertex.position * scale + instance.i_pos_scale.xyz;

    out.world_position = vec4<f32>(world_pos, 1.0);
    out.clip_position = mesh_view_bindings::view.clip_from_world * out.world_position;
    out.world_normal = vertex.normal;
    out.color = instance.i_color;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    let lighting = ambient + diffuse * 0.7;

    return vec4<f32>(in.color.rgb * lighting, in.color.a);
}

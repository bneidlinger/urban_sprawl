// Cloud shadows shader - procedural noise projected onto ground
// Creates drifting cloud shadow effect using fractal value noise

#import bevy_pbr::forward_io::VertexOutput

struct CloudShadowUniforms {
    scroll_offset: vec2<f32>,
    opacity: f32,
    scale: f32,
    coverage: f32,
    softness: f32,
    _padding: vec2<f32>,
};

@group(2) @binding(0)
var<uniform> uniforms: CloudShadowUniforms;

// Hash function for noise
fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let x = p * k + k.yx;
    return fract(16.0 * k.x * fract(x.x * x.y * (x.x + x.y)));
}

// Value noise
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Smooth interpolation
    let u = f * f * (3.0 - 2.0 * f);

    // Four corners
    let a = hash2(i + vec2<f32>(0.0, 0.0));
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));

    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractal Brownian Motion (layered noise)
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    // 5 octaves for cloud-like detail
    for (var i = 0; i < 5; i++) {
        value += amplitude * value_noise(pos * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
        // Rotate each octave slightly for more organic look
        pos = vec2<f32>(
            pos.x * 0.866 - pos.y * 0.5,
            pos.x * 0.5 + pos.y * 0.866
        );
    }

    return value;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample world position for noise (xz plane)
    let world_pos = in.world_position.xz;

    // Apply scroll offset and scale
    let sample_pos = (world_pos + uniforms.scroll_offset) * uniforms.scale;

    // Generate cloud pattern using FBM
    let noise = fbm(sample_pos);

    // Shape the noise into cloud-like shadows
    // coverage controls how much of the sky has clouds
    // softness controls the edge falloff
    let cloud = smoothstep(
        uniforms.coverage - uniforms.softness,
        uniforms.coverage + uniforms.softness,
        noise
    );

    // Shadow color (dark with slight blue tint for atmosphere)
    let shadow_color = vec3<f32>(0.0, 0.0, 0.05);

    // Final alpha based on cloud shape and overall opacity
    let alpha = cloud * uniforms.opacity;

    return vec4<f32>(shadow_color, alpha);
}

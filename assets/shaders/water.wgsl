// Water surface shader with animated waves
// Creates a flowing river effect with color variation and foam

#import bevy_pbr::forward_io::VertexOutput

struct WaterUniforms {
    time: f32,
    wave_speed: f32,
    wave_amplitude: f32,
    wave_frequency: f32,
    deep_color: vec4<f32>,
    shallow_color: vec4<f32>,
    foam_intensity: f32,
    opacity: f32,
};

@group(2) @binding(0)
var<uniform> uniforms: WaterUniforms;

// Hash function for noise
fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let x = p * k + k.yx;
    return fract(16.0 * k.x * fract(x.x * x.y * (x.x + x.y)));
}

// Value noise for foam
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash2(i + vec2<f32>(0.0, 0.0));
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Multiple overlapping wave functions
fn wave_displacement(pos: vec2<f32>, time: f32) -> f32 {
    let freq = uniforms.wave_frequency;
    let speed = uniforms.wave_speed;

    // Primary wave along river flow (V direction)
    let wave1 = sin(pos.y * freq * 2.0 + time * speed) * 0.5;

    // Secondary cross-wave
    let wave2 = sin(pos.x * freq * 3.0 + pos.y * freq + time * speed * 1.3) * 0.3;

    // Tertiary ripples
    let wave3 = sin((pos.x + pos.y) * freq * 4.0 + time * speed * 0.7) * 0.2;

    return (wave1 + wave2 + wave3) * uniforms.wave_amplitude;
}

// Calculate foam based on edge proximity and noise
fn calculate_foam(uv: vec2<f32>, time: f32) -> f32 {
    // Distance from edges (UV.x is 0 at left bank, 1 at right bank)
    let edge_dist = min(uv.x, 1.0 - uv.x) * 2.0;

    // Edge foam (stronger near banks)
    let edge_foam = 1.0 - smoothstep(0.0, 0.15, edge_dist);

    // Animated noise for foam texture
    let noise_pos = uv * 20.0 + vec2<f32>(time * 0.3, time * 0.5);
    let foam_noise = value_noise(noise_pos);

    // Combine edge proximity with noise
    return edge_foam * foam_noise * uniforms.foam_intensity;
}

// Fresnel-like effect for subtle reflection
fn fresnel_factor(view_dir: vec3<f32>, normal: vec3<f32>) -> f32 {
    let dot_product = max(dot(view_dir, normal), 0.0);
    return pow(1.0 - dot_product, 2.0) * 0.3;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position.xz;
    let uv = in.uv;

    // Calculate wave displacement for visual effect
    let wave = wave_displacement(world_pos * 0.1, uniforms.time);

    // Color interpolation: deep in center, shallow at edges
    let edge_factor = abs(uv.x - 0.5) * 2.0;
    let color_t = smoothstep(0.0, 0.4, edge_factor);
    var base_color = mix(uniforms.deep_color.rgb, uniforms.shallow_color.rgb, color_t);

    // Add wave-based color variation
    let wave_color_shift = wave * 0.3;
    base_color = base_color + vec3<f32>(wave_color_shift * 0.1, wave_color_shift * 0.15, wave_color_shift * 0.2);

    // Calculate foam
    let foam = calculate_foam(uv, uniforms.time);
    let foam_color = vec3<f32>(0.9, 0.95, 1.0);
    base_color = mix(base_color, foam_color, foam);

    // Subtle specular highlights based on wave peaks
    let highlight = smoothstep(0.3, 0.5, wave / uniforms.wave_amplitude) * 0.15;
    base_color = base_color + vec3<f32>(highlight);

    // Final alpha with slight variation
    let alpha = uniforms.opacity + foam * 0.1;

    return vec4<f32>(base_color, alpha);
}

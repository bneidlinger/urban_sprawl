// Wet surface overlay shader - adds reflective wetness to roads
// Creates puddles with reflections and rain ripples during rain

#import bevy_pbr::forward_io::VertexOutput

struct WetSurfaceUniforms {
    time: f32,
    wetness: f32,
    puddle_coverage: f32,
    reflection_strength: f32,
    sky_color: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> uniforms: WetSurfaceUniforms;

// Hash function for noise
fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let x = p * k + k.yx;
    return fract(16.0 * k.x * fract(x.x * x.y * (x.x + x.y)));
}

// Value noise for puddle shapes
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

// Rain ripple effect
fn ripple(uv: vec2<f32>, time: f32, seed: f32) -> f32 {
    // Random ripple center based on seed
    let center_hash = hash2(vec2<f32>(seed, seed * 1.7));
    let ripple_center = vec2<f32>(
        center_hash * 15.0 - 7.5,
        hash2(vec2<f32>(seed * 2.3, seed)) * 15.0 - 7.5
    );

    let dist = length(uv - ripple_center);

    // Ripple expands over time then fades
    let ripple_time = fract(time * 0.4 + seed * 0.3);
    let ripple_radius = ripple_time * 3.0;
    let ripple_width = 0.15;

    // Ring shape that fades as it expands
    let ring = smoothstep(ripple_width, 0.0, abs(dist - ripple_radius));
    let fade = 1.0 - ripple_time;

    return ring * fade * 0.3;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Early out if no wetness
    if uniforms.wetness < 0.01 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let world_pos = in.world_position.xz;
    let uv = world_pos * 0.04;

    // Generate puddle pattern using layered noise
    let noise_large = value_noise(uv * 2.0);
    let noise_small = value_noise(uv * 8.0);
    let puddle_noise = noise_large * 0.7 + noise_small * 0.3;

    // Puddle threshold based on coverage
    let puddle_threshold = 1.0 - uniforms.puddle_coverage;
    let is_puddle = smoothstep(puddle_threshold - 0.1, puddle_threshold + 0.1, puddle_noise);

    // Surface darkening from wetness
    let darken_amount = 0.12 * uniforms.wetness;

    // Reflection strength (stronger in puddles)
    let base_reflection = uniforms.reflection_strength * uniforms.wetness;
    let puddle_reflection = base_reflection * 2.5;
    let reflection = mix(base_reflection, puddle_reflection, is_puddle);

    // Add rain ripples to puddles when actively raining
    var ripple_effect = 0.0;
    if is_puddle > 0.3 && uniforms.wetness > 0.5 {
        // Multiple overlapping ripples
        for (var i = 0; i < 6; i++) {
            ripple_effect += ripple(world_pos * 0.25, uniforms.time, f32(i) * 7.3);
        }
    }

    // Sky reflection color
    let reflection_color = uniforms.sky_color.rgb * reflection;

    // Combine: darkening + reflection + ripples
    // Darkening is negative (subtractive), reflection and ripples are additive
    let surface_effect = -darken_amount + reflection_color.r * 0.5;
    let final_color = vec3<f32>(surface_effect) + vec3<f32>(ripple_effect * 0.15);

    // Shift to visible range (add 0.5 baseline for blend mode)
    let display_color = final_color + 0.5;

    // Alpha based on wetness and puddle factor
    let alpha = uniforms.wetness * 0.5 * (0.4 + is_puddle * 0.4);

    return vec4<f32>(display_color, alpha);
}

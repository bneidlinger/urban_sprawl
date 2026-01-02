// Rain particle shader - procedural falling raindrops
// Creates animated rain streaks using noise-based positioning

#import bevy_pbr::forward_io::VertexOutput

struct RainUniforms {
    time: f32,
    intensity: f32,
    speed: f32,
    angle: f32,
    opacity: f32,
    _padding: vec3<f32>,
};

@group(2) @binding(0)
var<uniform> uniforms: RainUniforms;

// Hash function for random drop positions
fn hash2(p: vec2<f32>) -> vec2<f32> {
    let k = vec2<f32>(0.3183099, 0.3678794);
    var n = p * k + k.yx;
    return fract(sin(vec2<f32>(
        dot(n, vec2<f32>(127.1, 311.7)),
        dot(n, vec2<f32>(269.5, 183.3))
    )) * 43758.5453);
}

// Generate a single raindrop at a grid cell
fn raindrop(cell: vec2<f32>, uv: vec2<f32>, time: f32) -> f32 {
    let rand = hash2(cell);

    // Random offset within cell
    let offset = rand * 0.8 + 0.1;

    // Drop falls with varied speed
    let fall_speed = uniforms.speed * (0.8 + rand.x * 0.4);
    let drop_y = fract(offset.y - time * fall_speed * 0.1);

    // Wind angle offset increases toward bottom of fall
    let wind_offset = sin(uniforms.angle) * (1.0 - drop_y) * 0.4;
    let drop_x = offset.x + wind_offset;

    // Local UV relative to drop center
    let local_uv = fract(uv) - vec2<f32>(drop_x, drop_y);

    // Elongated drop shape (streak)
    let drop_length = 0.15;
    let drop_width = 0.02;
    let aspect = vec2<f32>(1.0 / drop_width, 1.0 / drop_length);
    let dist = length(local_uv * aspect);

    // Fade based on intensity threshold (fewer drops at low intensity)
    let threshold = 1.0 - uniforms.intensity;
    if rand.x < threshold {
        return 0.0;
    }

    // Soft drop edge
    return smoothstep(1.0, 0.3, dist);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Early out if no rain
    if uniforms.intensity < 0.01 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let world_pos = in.world_position.xz;

    // Scale for raindrop density
    let density = 0.08;
    let uv = world_pos * density;

    var rain = 0.0;

    // Sample multiple layers for depth effect
    for (var layer = 0; layer < 3; layer++) {
        let layer_f = f32(layer);

        // Each layer has different scale and speed
        let layer_scale = 1.0 + layer_f * 0.4;
        let layer_speed = 1.0 - layer_f * 0.2;
        let layer_uv = uv * layer_scale;
        let layer_time = uniforms.time * layer_speed;

        let cell = floor(layer_uv);

        // Check surrounding cells for drops that might overlap
        for (var dx = -1; dx <= 1; dx++) {
            for (var dy = -1; dy <= 1; dy++) {
                let neighbor = cell + vec2<f32>(f32(dx), f32(dy));
                // Add layer offset to cell hash for variety
                let cell_id = neighbor + layer_f * 100.0;
                rain += raindrop(cell_id, layer_uv, layer_time);
            }
        }
    }

    // Rain color (white with slight blue tint)
    let rain_color = vec3<f32>(0.8, 0.85, 0.95);

    // Clamp and apply opacity
    let alpha = clamp(rain * 0.25 * uniforms.opacity, 0.0, 0.5);

    return vec4<f32>(rain_color, alpha);
}

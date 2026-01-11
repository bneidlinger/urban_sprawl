// Cinematic post-processing shader
// Applies film grain, vignette, and chromatic aberration for authentic film look

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct CinematicUniforms {
    grain_intensity: f32,
    grain_size: f32,
    vignette_intensity: f32,
    vignette_radius: f32,
    vignette_softness: f32,
    chromatic_intensity: f32,
    time: f32,
    _padding: f32,
};

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;
@group(0) @binding(2)
var<uniform> settings: CinematicUniforms;

// ============================================================================
// Hash functions for procedural noise
// ============================================================================

// High-quality hash for film grain
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = sin(dot(p, vec2<f32>(41.0, 289.0)));
    return fract(vec2<f32>(262144.0, 32768.0) * n);
}

// ============================================================================
// Film Grain
// ============================================================================

fn film_grain(uv: vec2<f32>, time: f32, intensity: f32, size: f32) -> f32 {
    // Use screen-resolution-based grain for consistent look
    // Animate grain by varying the input over time (slower animation)
    let animated_uv = uv * size * 500.0 + vec2<f32>(time * 37.0, time * 29.0);

    // Generate noise with temporal dithering for smoother look
    let noise1 = hash21(animated_uv);
    let noise2 = hash21(animated_uv + vec2<f32>(0.5, 0.5));
    let noise = (noise1 + noise2) * 0.5; // Average for smoother grain

    // Center around 0 and scale by intensity
    return (noise - 0.5) * intensity;
}

// ============================================================================
// Vignette
// ============================================================================

fn vignette(uv: vec2<f32>, intensity: f32, radius: f32, softness: f32) -> f32 {
    // Calculate distance from center (0.5, 0.5)
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(uv - center) * 2.0; // Normalize so corners are ~1.4

    // Apply smooth falloff
    let vignette_factor = 1.0 - smoothstep(radius - softness * 0.5, radius + softness * 0.5, dist);

    // Return darkening factor (1.0 = no darkening, 0.0 = full black)
    return mix(1.0 - intensity, 1.0, vignette_factor);
}

// ============================================================================
// Chromatic Aberration
// ============================================================================

fn chromatic_aberration(uv: vec2<f32>, intensity: f32) -> vec3<f32> {
    // Calculate offset direction (radial from center)
    let center = vec2<f32>(0.5, 0.5);
    let offset_dir = uv - center;

    // Scale offset by distance from center (more aberration at edges)
    let dist = length(offset_dir);
    let scaled_offset = offset_dir * intensity * dist;

    // Sample RGB channels at offset positions
    let r = textureSample(screen_texture, texture_sampler, uv + scaled_offset).r;
    let g = textureSample(screen_texture, texture_sampler, uv).g;
    let b = textureSample(screen_texture, texture_sampler, uv - scaled_offset).b;

    return vec3<f32>(r, g, b);
}

// ============================================================================
// Main Fragment Shader
// ============================================================================

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Start with original color or chromatic aberration
    var color: vec3<f32>;

    if (settings.chromatic_intensity > 0.0001) {
        color = chromatic_aberration(uv, settings.chromatic_intensity);
    } else {
        color = textureSample(screen_texture, texture_sampler, uv).rgb;
    }

    // Apply vignette
    if (settings.vignette_intensity > 0.0001) {
        let vignette_factor = vignette(
            uv,
            settings.vignette_intensity,
            settings.vignette_radius,
            settings.vignette_softness
        );
        color = color * vignette_factor;
    }

    // Apply film grain (additive, subtle)
    if (settings.grain_intensity > 0.0001) {
        let grain = film_grain(uv, settings.time, settings.grain_intensity, settings.grain_size);

        // Apply grain uniformly - avoid amplifying in shadows which creates static look
        // Real film grain is actually less visible in very dark areas
        let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
        let grain_visibility = smoothstep(0.0, 0.3, luminance); // Reduce grain in very dark areas

        color = color + vec3<f32>(grain * grain_visibility);
    }

    // Clamp to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(color, 1.0);
}

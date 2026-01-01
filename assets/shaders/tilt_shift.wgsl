// Tilt-shift post-processing shader
// Creates a miniature/diorama effect by blurring top and bottom of screen

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct TiltShiftUniforms {
    focus_center: f32,
    focus_width: f32,
    blur_amount: f32,
    blur_samples: i32,
    saturation: f32,
    _padding: f32,
};

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;
@group(0) @binding(2)
var<uniform> settings: TiltShiftUniforms;

// Calculate blur strength based on vertical position
fn calculate_blur_factor(uv_y: f32) -> f32 {
    // Distance from focus center
    let distance = abs(uv_y - settings.focus_center);

    // Calculate blur factor (0 in focus band, increasing outside)
    let half_width = settings.focus_width * 0.5;
    let blur_factor = smoothstep(half_width * 0.5, half_width, distance);

    return blur_factor;
}

// Apply saturation adjustment
fn adjust_saturation(color: vec3<f32>, saturation: f32) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return mix(vec3<f32>(luminance), color, saturation);
}

// Gaussian weight approximation
fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let texel_size = 1.0 / texture_size;

    // Calculate blur strength at this pixel
    let blur_factor = calculate_blur_factor(uv.y);
    let blur_radius = settings.blur_amount * blur_factor;

    // Early out if no blur needed
    if (blur_radius < 0.5) {
        let color = textureSample(screen_texture, texture_sampler, uv);
        let saturated = adjust_saturation(color.rgb, settings.saturation);
        return vec4<f32>(saturated, color.a);
    }

    // Perform horizontal + vertical blur (separable approximation in single pass)
    var color_sum = vec3<f32>(0.0);
    var weight_sum = 0.0;

    let samples = settings.blur_samples;
    let sigma = blur_radius * 0.5;

    // Sample in a cross pattern for efficiency
    for (var i = -samples; i <= samples; i++) {
        let offset = f32(i);
        let weight = gaussian_weight(offset, sigma);

        // Horizontal samples
        let h_uv = uv + vec2<f32>(offset * texel_size.x * blur_radius, 0.0);
        color_sum += textureSample(screen_texture, texture_sampler, h_uv).rgb * weight;
        weight_sum += weight;

        // Vertical samples (skip center to avoid double-counting)
        if (i != 0) {
            let v_uv = uv + vec2<f32>(0.0, offset * texel_size.y * blur_radius);
            color_sum += textureSample(screen_texture, texture_sampler, v_uv).rgb * weight;
            weight_sum += weight;
        }
    }

    var final_color = color_sum / weight_sum;

    // Apply saturation boost for miniature effect
    final_color = adjust_saturation(final_color, settings.saturation);

    return vec4<f32>(final_color, 1.0);
}

// HZB (Hierarchical Z-Buffer) Generation Compute Shader
//
// Generates a depth pyramid by downsampling the depth buffer using MAX reduction.
// Each mip level stores the maximum depth from the 2x2 region below it.
//
// This enables efficient occlusion queries by sampling at an appropriate mip level
// based on the object's screen-space size.

struct HzbUniforms {
    input_size: vec2<u32>,
    output_size: vec2<u32>,
    src_mip: u32,
    dst_mip: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var input_depth: texture_2d<f32>;
@group(0) @binding(1) var output_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var<uniform> uniforms: HzbUniforms;

// Workgroup size: 8x8 threads
@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let output_coord = global_id.xy;

    // Early exit if outside output dimensions
    if (output_coord.x >= uniforms.output_size.x || output_coord.y >= uniforms.output_size.y) {
        return;
    }

    // Calculate input coordinates (2x2 region)
    let input_base = output_coord * 2u;

    // Sample 2x2 region and take maximum
    var max_depth = 0.0;

    for (var dy = 0u; dy < 2u; dy++) {
        for (var dx = 0u; dx < 2u; dx++) {
            let sample_coord = vec2<i32>(
                i32(min(input_base.x + dx, uniforms.input_size.x - 1u)),
                i32(min(input_base.y + dy, uniforms.input_size.y - 1u))
            );

            let depth = textureLoad(input_depth, sample_coord, 0).r;
            max_depth = max(max_depth, depth);
        }
    }

    // Write to output mip level
    textureStore(output_depth, vec2<i32>(output_coord), vec4<f32>(max_depth, 0.0, 0.0, 1.0));
}

// Alternative: Single-pass multi-mip generation using shared memory
// This version generates multiple mip levels in one dispatch for better efficiency

// Shared memory for hierarchical reduction
var<workgroup> shared_depth: array<f32, 64>; // 8x8 workgroup

@compute @workgroup_size(8, 8, 1)
fn generate_mips_shared(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let local_idx = local_id.y * 8u + local_id.x;

    // Load 4 samples per thread (covers 16x16 input region per workgroup)
    var input_coord = global_id.xy * 2u;

    var local_max = 0.0;
    for (var dy = 0u; dy < 2u; dy++) {
        for (var dx = 0u; dx < 2u; dx++) {
            let sample_coord = vec2<i32>(
                i32(min(input_coord.x + dx, uniforms.input_size.x - 1u)),
                i32(min(input_coord.y + dy, uniforms.input_size.y - 1u))
            );
            let depth = textureLoad(input_depth, sample_coord, 0).r;
            local_max = max(local_max, depth);
        }
    }

    // Store to shared memory
    shared_depth[local_idx] = local_max;
    workgroupBarrier();

    // Write mip level 1 (every thread writes one pixel)
    if (global_id.x < uniforms.output_size.x && global_id.y < uniforms.output_size.y) {
        textureStore(output_depth, vec2<i32>(global_id.xy), vec4<f32>(local_max, 0.0, 0.0, 1.0));
    }

    // Reduction for mip level 2 (4x4 threads)
    if (local_id.x < 4u && local_id.y < 4u) {
        let base = local_id.y * 2u * 8u + local_id.x * 2u;
        let max4 = max(
            max(shared_depth[base], shared_depth[base + 1u]),
            max(shared_depth[base + 8u], shared_depth[base + 9u])
        );
        shared_depth[local_idx] = max4;
    }
    workgroupBarrier();

    // Reduction for mip level 3 (2x2 threads)
    if (local_id.x < 2u && local_id.y < 2u) {
        let base = local_id.y * 2u * 8u + local_id.x * 2u;
        let max4 = max(
            max(shared_depth[base], shared_depth[base + 1u]),
            max(shared_depth[base + 8u], shared_depth[base + 9u])
        );
        shared_depth[local_idx] = max4;
    }
    workgroupBarrier();

    // Reduction for mip level 4 (1 thread)
    if (local_id.x == 0u && local_id.y == 0u) {
        let final_max = max(
            max(shared_depth[0], shared_depth[1]),
            max(shared_depth[8], shared_depth[9])
        );
        // This would write to a higher mip level
        // Actual implementation would need additional storage textures
    }
}

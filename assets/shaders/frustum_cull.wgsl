// GPU Frustum Culling Compute Shader with HZB Occlusion
//
// Tests objects against:
// 1. View frustum (6 plane tests)
// 2. HZB depth pyramid (occlusion culling)
//
// Each workgroup processes 64 objects in parallel.
//
// Input:
// - Object data buffer (bounding spheres + metadata)
// - Frustum planes (6 planes from view-projection matrix)
// - HZB pyramid texture (optional, for occlusion culling)
// - View-projection matrix (for screen-space projection)
//
// Output:
// - Visibility buffer (1 = visible, 0 = culled per object)
// - Optional: Indirect draw commands with instance_count updated

// Frustum plane: ax + by + cz + d = 0
// Normal (a,b,c) points inward toward visible space
struct Plane {
    coefficients: vec4<f32>,  // (a, b, c, d)
}

// 6 frustum planes: Left, Right, Bottom, Top, Near, Far
struct FrustumPlanes {
    left: Plane,
    right: Plane,
    bottom: Plane,
    top: Plane,
    near: Plane,
    far: Plane,
}

// Per-object data (32 bytes, aligned)
struct ObjectData {
    bounding_sphere: vec4<f32>,  // xyz = center, w = radius
    entity_bits: vec2<u32>,       // 64-bit entity ID split into two u32
    mesh_id: u32,
    flags: u32,                   // bit 0 = visible
}

// Indirect draw command (for GPU-driven rendering)
struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: u32,  // 0 = culled, 1 = visible
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

// Uniforms
struct CullUniforms {
    object_count: u32,
    frustum_padding: f32,
    hzb_enabled: u32,        // 1 = HZB occlusion enabled
    min_screen_size: f32,    // Minimum screen size to test HZB
}

// HZB uniforms for occlusion testing
struct HzbUniforms {
    view_proj: mat4x4<f32>,      // View-projection matrix
    screen_size: vec2<f32>,      // Screen dimensions
    mip_count: u32,              // Number of HZB mip levels
    depth_bias: f32,             // Conservative depth bias
    near_plane: f32,
    far_plane: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: CullUniforms;
@group(0) @binding(1) var<storage, read> frustum: FrustumPlanes;
@group(0) @binding(2) var<storage, read> objects: array<ObjectData>;
@group(0) @binding(3) var<storage, read_write> visibility: array<u32>;
@group(0) @binding(4) var<storage, read_write> draw_commands: array<DrawIndexedIndirect>;

// HZB bindings (group 1 for optional HZB support)
@group(1) @binding(0) var<uniform> hzb_uniforms: HzbUniforms;
@group(1) @binding(1) var hzb_pyramid: texture_2d<f32>;
@group(1) @binding(2) var hzb_sampler: sampler;

// Workgroup size: 64 threads per workgroup
@compute @workgroup_size(64, 1, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let object_index = global_id.x;

    // Early exit if beyond object count
    if (object_index >= uniforms.object_count) {
        return;
    }

    // Load object data
    let object = objects[object_index];
    let center = object.bounding_sphere.xyz;
    let radius = object.bounding_sphere.w + uniforms.frustum_padding;

    // Test against all 6 frustum planes
    var visible = true;

    // Left plane
    if (plane_sphere_test(frustum.left, center, radius) < 0.0) {
        visible = false;
    }
    // Right plane
    if (visible && plane_sphere_test(frustum.right, center, radius) < 0.0) {
        visible = false;
    }
    // Bottom plane
    if (visible && plane_sphere_test(frustum.bottom, center, radius) < 0.0) {
        visible = false;
    }
    // Top plane
    if (visible && plane_sphere_test(frustum.top, center, radius) < 0.0) {
        visible = false;
    }
    // Near plane
    if (visible && plane_sphere_test(frustum.near, center, radius) < 0.0) {
        visible = false;
    }
    // Far plane
    if (visible && plane_sphere_test(frustum.far, center, radius) < 0.0) {
        visible = false;
    }

    // Write visibility result
    visibility[object_index] = select(0u, 1u, visible);
}

// Main entry point for indirect draw command generation
// Updates both visibility buffer and indirect draw commands
@compute @workgroup_size(64, 1, 1)
fn main_indirect(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let object_index = global_id.x;

    // Early exit if beyond object count
    if (object_index >= uniforms.object_count) {
        return;
    }

    // Load object data
    let object = objects[object_index];
    let center = object.bounding_sphere.xyz;
    let radius = object.bounding_sphere.w + uniforms.frustum_padding;

    // Test against all 6 frustum planes
    var visible = true;

    // Left plane
    if (plane_sphere_test(frustum.left, center, radius) < 0.0) {
        visible = false;
    }
    // Right plane
    if (visible && plane_sphere_test(frustum.right, center, radius) < 0.0) {
        visible = false;
    }
    // Bottom plane
    if (visible && plane_sphere_test(frustum.bottom, center, radius) < 0.0) {
        visible = false;
    }
    // Top plane
    if (visible && plane_sphere_test(frustum.top, center, radius) < 0.0) {
        visible = false;
    }
    // Near plane
    if (visible && plane_sphere_test(frustum.near, center, radius) < 0.0) {
        visible = false;
    }
    // Far plane
    if (visible && plane_sphere_test(frustum.far, center, radius) < 0.0) {
        visible = false;
    }

    // Write visibility result
    let vis_flag = select(0u, 1u, visible);
    visibility[object_index] = vis_flag;

    // Update indirect draw command instance_count
    // 0 = culled (no instances), 1 = visible (draw 1 instance)
    draw_commands[object_index].instance_count = vis_flag;
}

// Test sphere against plane
// Returns signed distance from sphere surface to plane
// Positive = sphere is on visible side or intersecting
// Negative = sphere is completely behind plane (culled)
fn plane_sphere_test(plane: Plane, center: vec3<f32>, radius: f32) -> f32 {
    let normal = plane.coefficients.xyz;
    let d = plane.coefficients.w;

    // Signed distance from center to plane
    let distance = dot(normal, center) + d;

    // Subtract radius to get distance from sphere surface
    return distance + radius;
}

// Alternative: AABB frustum test (for future use)
fn test_aabb_frustum(min: vec3<f32>, max: vec3<f32>) -> bool {
    let planes = array<Plane, 6>(
        frustum.left,
        frustum.right,
        frustum.bottom,
        frustum.top,
        frustum.near,
        frustum.far
    );

    for (var i = 0u; i < 6u; i++) {
        let plane = planes[i];
        let n = plane.coefficients.xyz;

        // Find the positive vertex (furthest in direction of normal)
        let p_vertex = vec3<f32>(
            select(min.x, max.x, n.x >= 0.0),
            select(min.y, max.y, n.y >= 0.0),
            select(min.z, max.z, n.z >= 0.0)
        );

        // If positive vertex is behind plane, AABB is outside
        if (dot(n, p_vertex) + plane.coefficients.w < 0.0) {
            return false;
        }
    }

    return true;
}

// Utility: Convert object flags to visibility
fn is_object_enabled(flags: u32) -> bool {
    return (flags & 1u) != 0u;
}

// ============================================================================
// HZB Occlusion Culling Functions
// ============================================================================

// Project a world-space point to clip space
fn project_point(world_pos: vec3<f32>) -> vec4<f32> {
    return hzb_uniforms.view_proj * vec4<f32>(world_pos, 1.0);
}

// Convert clip space to NDC
fn clip_to_ndc(clip: vec4<f32>) -> vec3<f32> {
    return clip.xyz / clip.w;
}

// Convert NDC to UV coordinates [0, 1]
fn ndc_to_uv(ndc: vec3<f32>) -> vec2<f32> {
    return vec2<f32>(
        (ndc.x + 1.0) * 0.5,
        (1.0 - ndc.y) * 0.5  // Flip Y for texture coordinates
    );
}

// Calculate appropriate HZB mip level based on screen-space size
fn calculate_hzb_mip(screen_size_pixels: f32) -> u32 {
    if (screen_size_pixels <= 0.0) {
        return hzb_uniforms.mip_count - 1u;
    }

    // We want to sample a mip where one texel roughly covers the object
    let base_size = max(hzb_uniforms.screen_size.x, hzb_uniforms.screen_size.y);
    let ideal_mip = u32(ceil(log2(base_size / screen_size_pixels)));
    return min(ideal_mip, hzb_uniforms.mip_count - 1u);
}

// Project bounding sphere to screen space and get its size
fn project_sphere_screen_size(center: vec3<f32>, radius: f32) -> f32 {
    let clip_center = project_point(center);

    // Behind camera
    if (clip_center.w <= 0.0) {
        return 0.0;
    }

    // Project a point at the edge of the sphere (perpendicular to view)
    // Approximate by projecting center +/- radius in screen space
    let ndc_center = clip_to_ndc(clip_center);

    // Calculate projected radius in NDC space
    // This is an approximation: actual sphere projection is complex
    let projected_radius = radius / clip_center.w;

    // Convert to screen pixels
    let screen_radius = projected_radius * max(hzb_uniforms.screen_size.x, hzb_uniforms.screen_size.y) * 0.5;

    return screen_radius * 2.0; // Diameter
}

// Sample the HZB pyramid at a given UV and mip level
fn sample_hzb(uv: vec2<f32>, mip: u32) -> f32 {
    return textureSampleLevel(hzb_pyramid, hzb_sampler, uv, f32(mip)).r;
}

// Test if a bounding sphere is occluded by the HZB
// Returns true if the object is occluded (should be culled)
fn hzb_occlusion_test(center: vec3<f32>, radius: f32) -> bool {
    // Project sphere center to clip space
    let clip = project_point(center);

    // Behind camera - not occluded (will be frustum culled)
    if (clip.w <= 0.0) {
        return false;
    }

    let ndc = clip_to_ndc(clip);

    // Outside NDC bounds - not occluded (will be frustum culled)
    if (ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0) {
        return false;
    }

    // Get screen-space size
    let screen_size = project_sphere_screen_size(center, radius);

    // Skip HZB test for very small objects (may cause false positives)
    if (screen_size < uniforms.min_screen_size) {
        return false;
    }

    // Calculate UV coordinates
    let uv = ndc_to_uv(ndc);

    // Calculate appropriate mip level
    let mip = calculate_hzb_mip(screen_size);

    // Sample HZB depth
    let hzb_depth = sample_hzb(uv, mip);

    // Calculate object's near depth (front of bounding sphere)
    // We use the point on the sphere closest to the camera
    let near_clip = project_point(center - vec3<f32>(0.0, 0.0, radius));
    var object_near_depth = ndc.z;
    if (near_clip.w > 0.0) {
        object_near_depth = near_clip.z / near_clip.w;
    }

    // Object is occluded if its near depth > HZB depth + bias
    // (depth buffer stores larger values for farther objects in Vulkan)
    return object_near_depth > hzb_depth + hzb_uniforms.depth_bias;
}

// Main entry point with HZB occlusion culling
@compute @workgroup_size(64, 1, 1)
fn main_with_hzb(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let object_index = global_id.x;

    // Early exit if beyond object count
    if (object_index >= uniforms.object_count) {
        return;
    }

    // Load object data
    let object = objects[object_index];
    let center = object.bounding_sphere.xyz;
    let radius = object.bounding_sphere.w + uniforms.frustum_padding;

    // First: Frustum culling (fast rejection)
    var visible = true;

    if (plane_sphere_test(frustum.left, center, radius) < 0.0) {
        visible = false;
    }
    if (visible && plane_sphere_test(frustum.right, center, radius) < 0.0) {
        visible = false;
    }
    if (visible && plane_sphere_test(frustum.bottom, center, radius) < 0.0) {
        visible = false;
    }
    if (visible && plane_sphere_test(frustum.top, center, radius) < 0.0) {
        visible = false;
    }
    if (visible && plane_sphere_test(frustum.near, center, radius) < 0.0) {
        visible = false;
    }
    if (visible && plane_sphere_test(frustum.far, center, radius) < 0.0) {
        visible = false;
    }

    // Second: HZB occlusion culling (only if frustum test passed)
    if (visible && uniforms.hzb_enabled == 1u) {
        if (hzb_occlusion_test(center, radius)) {
            visible = false;
        }
    }

    // Write visibility result
    visibility[object_index] = select(0u, 1u, visible);
}

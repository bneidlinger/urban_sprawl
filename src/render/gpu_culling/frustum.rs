//! Frustum plane extraction and testing.
//!
//! Extracts the 6 frustum planes from a view-projection matrix
//! for use in culling operations.

use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// A plane in 3D space represented as ax + by + cz + d = 0.
///
/// The normal is (a, b, c) and d is the distance from origin.
/// For frustum planes, the normal points inward (toward visible space).
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
#[repr(C)]
pub struct Plane {
    /// Plane equation coefficients: (a, b, c, d) where ax + by + cz + d = 0
    pub coefficients: [f32; 4],
}

impl Plane {
    /// Create a new plane from coefficients.
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self {
            coefficients: [a, b, c, d],
        }
    }

    /// Create a plane from a normal and distance.
    pub fn from_normal_distance(normal: Vec3, distance: f32) -> Self {
        Self::new(normal.x, normal.y, normal.z, distance)
    }

    /// Get the plane normal (not normalized).
    pub fn normal(&self) -> Vec3 {
        Vec3::new(self.coefficients[0], self.coefficients[1], self.coefficients[2])
    }

    /// Get the d coefficient.
    pub fn d(&self) -> f32 {
        self.coefficients[3]
    }

    /// Normalize the plane (make normal unit length).
    pub fn normalize(&mut self) {
        let length = self.normal().length();
        if length > 0.0 {
            self.coefficients[0] /= length;
            self.coefficients[1] /= length;
            self.coefficients[2] /= length;
            self.coefficients[3] /= length;
        }
    }

    /// Get the signed distance from a point to the plane.
    ///
    /// Positive = in front of plane (visible side)
    /// Negative = behind plane (culled side)
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal().dot(point) + self.d()
    }

    /// Test if a sphere is in front of or intersecting the plane.
    ///
    /// Returns true if any part of the sphere is on the visible side.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.signed_distance(center) >= -radius
    }
}

/// The 6 planes of a view frustum.
///
/// Planes are ordered: Left, Right, Bottom, Top, Near, Far.
/// All plane normals point inward (toward the visible volume).
#[derive(Resource, Clone, Copy, Pod, Zeroable, Debug, Default)]
#[repr(C)]
pub struct FrustumPlanes {
    /// Left frustum plane
    pub left: Plane,
    /// Right frustum plane
    pub right: Plane,
    /// Bottom frustum plane
    pub bottom: Plane,
    /// Top frustum plane
    pub top: Plane,
    /// Near frustum plane
    pub near: Plane,
    /// Far frustum plane
    pub far: Plane,
}

impl FrustumPlanes {
    /// Test if a sphere is inside or intersecting the frustum.
    ///
    /// Returns true if any part of the sphere is visible.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        // Test against all 6 planes
        // If the sphere is completely behind any plane, it's culled
        self.left.test_sphere(center, radius)
            && self.right.test_sphere(center, radius)
            && self.bottom.test_sphere(center, radius)
            && self.top.test_sphere(center, radius)
            && self.near.test_sphere(center, radius)
            && self.far.test_sphere(center, radius)
    }

    /// Test if a point is inside the frustum.
    pub fn test_point(&self, point: Vec3) -> bool {
        self.test_sphere(point, 0.0)
    }

    /// Test if an axis-aligned bounding box is inside or intersecting the frustum.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        // For each plane, find the corner that's most in the direction of the plane normal
        // If that corner is behind the plane, the AABB is culled
        let planes = [
            &self.left,
            &self.right,
            &self.bottom,
            &self.top,
            &self.near,
            &self.far,
        ];

        for plane in planes {
            // Find the positive vertex (furthest in direction of normal)
            let n = plane.normal();
            let p_vertex = Vec3::new(
                if n.x >= 0.0 { max.x } else { min.x },
                if n.y >= 0.0 { max.y } else { min.y },
                if n.z >= 0.0 { max.z } else { min.z },
            );

            // If the positive vertex is behind the plane, AABB is outside
            if plane.signed_distance(p_vertex) < 0.0 {
                return false;
            }
        }

        true
    }

    /// Get all 6 planes as an array.
    pub fn as_array(&self) -> [Plane; 6] {
        [
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        ]
    }

    /// Get raw bytes for GPU upload (6 planes * 4 floats * 4 bytes = 96 bytes).
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// Extract frustum planes from a view-projection matrix.
///
/// Uses the Gribb-Hartmann method for extracting planes from the combined
/// view-projection matrix. The resulting planes have inward-pointing normals.
///
/// Reference: https://www.gamedevs.org/uploads/fast-extraction-viewing-frustum-planes-from-world-view-projection-matrix.pdf
pub fn extract_frustum_planes(view_proj: Mat4) -> FrustumPlanes {
    let m = view_proj;

    // Extract rows of the matrix
    let row0 = Vec4::new(m.x_axis.x, m.y_axis.x, m.z_axis.x, m.w_axis.x);
    let row1 = Vec4::new(m.x_axis.y, m.y_axis.y, m.z_axis.y, m.w_axis.y);
    let row2 = Vec4::new(m.x_axis.z, m.y_axis.z, m.z_axis.z, m.w_axis.z);
    let row3 = Vec4::new(m.x_axis.w, m.y_axis.w, m.z_axis.w, m.w_axis.w);

    // Extract planes (Gribb-Hartmann method)
    // Left:   row3 + row0
    // Right:  row3 - row0
    // Bottom: row3 + row1
    // Top:    row3 - row1
    // Near:   row3 + row2 (for OpenGL-style NDC with z in [-1, 1])
    // Far:    row3 - row2

    // For Vulkan/DirectX-style NDC with z in [0, 1], near plane is just row2
    // Bevy uses Vulkan conventions

    let mut left = Plane::new(
        row3.x + row0.x,
        row3.y + row0.y,
        row3.z + row0.z,
        row3.w + row0.w,
    );
    let mut right = Plane::new(
        row3.x - row0.x,
        row3.y - row0.y,
        row3.z - row0.z,
        row3.w - row0.w,
    );
    let mut bottom = Plane::new(
        row3.x + row1.x,
        row3.y + row1.y,
        row3.z + row1.z,
        row3.w + row1.w,
    );
    let mut top = Plane::new(
        row3.x - row1.x,
        row3.y - row1.y,
        row3.z - row1.z,
        row3.w - row1.w,
    );
    // Near plane for Vulkan NDC (z in [0, 1])
    let mut near = Plane::new(row2.x, row2.y, row2.z, row2.w);
    let mut far = Plane::new(
        row3.x - row2.x,
        row3.y - row2.y,
        row3.z - row2.z,
        row3.w - row2.w,
    );

    // Normalize all planes
    left.normalize();
    right.normalize();
    bottom.normalize();
    top.normalize();
    near.normalize();
    far.normalize();

    FrustumPlanes {
        left,
        right,
        bottom,
        top,
        near,
        far,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_sphere_intersection() {
        // Plane at z=0, normal pointing +z
        let plane = Plane::new(0.0, 0.0, 1.0, 0.0);

        // Sphere in front of plane
        assert!(plane.test_sphere(Vec3::new(0.0, 0.0, 5.0), 1.0));

        // Sphere behind plane
        assert!(!plane.test_sphere(Vec3::new(0.0, 0.0, -5.0), 1.0));

        // Sphere intersecting plane
        assert!(plane.test_sphere(Vec3::new(0.0, 0.0, 0.5), 1.0));
        assert!(plane.test_sphere(Vec3::new(0.0, 0.0, -0.5), 1.0));
    }

    #[test]
    fn test_frustum_extraction() {
        // Simple orthographic projection
        let proj = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let view_proj = proj * view;

        let frustum = extract_frustum_planes(view_proj);

        // Point at origin should be inside frustum
        assert!(frustum.test_point(Vec3::ZERO));

        // Point far behind camera should be outside
        assert!(!frustum.test_point(Vec3::new(0.0, 0.0, 10.0)));
    }
}

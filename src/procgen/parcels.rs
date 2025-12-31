//! Parcel subdivision using OBB and straight skeleton algorithms.
//!
//! Converts city blocks into buildable lots.

#![allow(dead_code)]

use bevy::prelude::*;

/// A city block (closed polygon from road network cycles).
#[derive(Clone, Debug)]
pub struct Block {
    pub vertices: Vec<Vec2>,
    pub area: f32,
}

impl Block {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        let area = Self::calculate_area(&vertices);
        Self { vertices, area }
    }

    fn calculate_area(vertices: &[Vec2]) -> f32 {
        // Shoelace formula
        let n = vertices.len();
        if n < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            area += vertices[i].x * vertices[j].y;
            area -= vertices[j].x * vertices[i].y;
        }

        area.abs() / 2.0
    }

    /// Calculate the oriented bounding box (OBB).
    pub fn compute_obb(&self) -> OrientedBoundingBox {
        // Use rotating calipers for exact OBB (simplified version here)
        // For now, use PCA-based approach

        // Calculate centroid
        let centroid: Vec2 = self.vertices.iter().copied().sum::<Vec2>() / self.vertices.len() as f32;

        // Calculate covariance matrix
        let mut cxx = 0.0;
        let mut cyy = 0.0;
        let mut cxy = 0.0;

        for v in &self.vertices {
            let d = *v - centroid;
            cxx += d.x * d.x;
            cyy += d.y * d.y;
            cxy += d.x * d.y;
        }

        // Principal axis from eigenvector
        let angle = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        let axis = Vec2::new(angle.cos(), angle.sin());

        // Project vertices onto axes
        let perp = Vec2::new(-axis.y, axis.x);
        let mut min_major = f32::MAX;
        let mut max_major = f32::MIN;
        let mut min_minor = f32::MAX;
        let mut max_minor = f32::MIN;

        for v in &self.vertices {
            let d = *v - centroid;
            let proj_major = d.dot(axis);
            let proj_minor = d.dot(perp);

            min_major = min_major.min(proj_major);
            max_major = max_major.max(proj_major);
            min_minor = min_minor.min(proj_minor);
            max_minor = max_minor.max(proj_minor);
        }

        OrientedBoundingBox {
            center: centroid,
            half_extents: Vec2::new(
                (max_major - min_major) / 2.0,
                (max_minor - min_minor) / 2.0,
            ),
            rotation: angle,
        }
    }
}

/// An oriented bounding box.
#[derive(Clone, Copy, Debug)]
pub struct OrientedBoundingBox {
    pub center: Vec2,
    pub half_extents: Vec2,
    pub rotation: f32,
}

/// A buildable lot (subdivision result).
#[derive(Clone, Debug)]
pub struct Lot {
    pub vertices: Vec<Vec2>,
    pub area: f32,
    pub frontage: Option<LotFrontage>,
}

/// Information about which side of the lot faces the street.
#[derive(Clone, Debug)]
pub struct LotFrontage {
    pub edge_index: usize,
    pub street_width: f32,
}

/// Configuration for subdivision.
#[derive(Clone, Debug)]
pub struct SubdivisionConfig {
    pub min_lot_area: f32,
    pub max_lot_area: f32,
    pub min_lot_width: f32,
}

impl Default for SubdivisionConfig {
    fn default() -> Self {
        Self {
            min_lot_area: 200.0,  // 200 m²
            max_lot_area: 800.0,  // 800 m²
            min_lot_width: 10.0,  // 10 m
        }
    }
}

/// Subdivide a block into lots using OBB recursive splitting.
pub fn subdivide_block(block: &Block, config: &SubdivisionConfig) -> Vec<Lot> {
    let mut result = Vec::new();
    subdivide_recursive(&block.vertices, config, &mut result);
    result
}

fn subdivide_recursive(vertices: &[Vec2], config: &SubdivisionConfig, result: &mut Vec<Lot>) {
    let area = Block::calculate_area(vertices);

    // If small enough, it's a lot
    if area <= config.max_lot_area {
        result.push(Lot {
            vertices: vertices.to_vec(),
            area,
            frontage: None, // TODO: Determine from adjacent roads
        });
        return;
    }

    // Calculate OBB
    let block = Block::new(vertices.to_vec());
    let obb = block.compute_obb();

    // Split perpendicular to the longer axis
    let split_axis = if obb.half_extents.x > obb.half_extents.y {
        Vec2::new(obb.rotation.cos(), obb.rotation.sin())
    } else {
        Vec2::new(-obb.rotation.sin(), obb.rotation.cos())
    };

    // Split line through center
    let (left, right) = split_polygon(vertices, obb.center, split_axis);

    if left.len() >= 3 && right.len() >= 3 {
        subdivide_recursive(&left, config, result);
        subdivide_recursive(&right, config, result);
    } else {
        // Can't split further
        result.push(Lot {
            vertices: vertices.to_vec(),
            area,
            frontage: None,
        });
    }
}

/// Split a polygon by a line (point + direction).
fn split_polygon(vertices: &[Vec2], point: Vec2, direction: Vec2) -> (Vec<Vec2>, Vec<Vec2>) {
    let normal = Vec2::new(-direction.y, direction.x);
    let mut left = Vec::new();
    let mut right = Vec::new();

    let n = vertices.len();
    for i in 0..n {
        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % n];

        let d1 = (v1 - point).dot(normal);
        let d2 = (v2 - point).dot(normal);

        if d1 >= 0.0 {
            left.push(v1);
        }
        if d1 <= 0.0 {
            right.push(v1);
        }

        // Check for intersection
        if (d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0) {
            let t = d1 / (d1 - d2);
            let intersection = v1 + (v2 - v1) * t;
            left.push(intersection);
            right.push(intersection);
        }
    }

    (left, right)
}

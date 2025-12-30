use bevy::prelude::*;

/// Calculate the centroid of a lot polygon.
pub fn lot_centroid(vertices: &[Vec2]) -> Vec2 {
    if vertices.is_empty() {
        return Vec2::ZERO;
    }

    vertices.iter().copied().sum::<Vec2>() / vertices.len() as f32
}

/// Shrink a polygon toward its centroid by a fixed distance to create a setback footprint.
pub fn shrink_polygon(vertices: &[Vec2], distance: f32) -> Vec<Vec2> {
    if vertices.len() < 3 {
        return Vec::new();
    }

    let centroid = lot_centroid(vertices);

    vertices
        .iter()
        .map(|&v| {
            let dir = (v - centroid).normalize_or_zero();
            v - dir * distance
        })
        .collect()
}

/// Compute axis-aligned bounding box of a polygon.
pub fn polygon_bounds(vertices: &[Vec2]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);

    for &v in vertices {
        min = min.min(v);
        max = max.max(v);
    }

    (min, max)
}

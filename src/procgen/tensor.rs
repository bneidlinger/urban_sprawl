//! Tensor field implementation for road network generation.
//!
//! Reference: Chen et al. 2008 - "Interactive Procedural Street Modeling"
//! https://www.sci.utah.edu/~chengu/street_sig08/street_sig08.pdf

use bevy::prelude::*;

pub struct TensorFieldPlugin;

impl Plugin for TensorFieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TensorField>();
    }
}

/// A 2x2 symmetric tensor representing road orientation at a point.
#[derive(Clone, Copy, Debug)]
pub struct Tensor {
    /// The tensor components: [[r, s], [s, -r]] where eigenvectors give road directions
    pub r: f32,
    pub s: f32,
}

impl Tensor {
    pub fn new(r: f32, s: f32) -> Self {
        Self { r, s }
    }

    /// Create a tensor aligned to a direction vector.
    pub fn from_direction(dir: Vec2) -> Self {
        let angle = dir.y.atan2(dir.x);
        Self::from_angle(angle)
    }

    /// Create a tensor from an angle (radians).
    pub fn from_angle(theta: f32) -> Self {
        Self {
            r: (2.0 * theta).cos(),
            s: (2.0 * theta).sin(),
        }
    }

    /// Get the major eigenvector (primary road direction).
    pub fn major(&self) -> Vec2 {
        let theta = 0.5 * self.s.atan2(self.r);
        Vec2::new(theta.cos(), theta.sin())
    }

    /// Get the minor eigenvector (cross-street direction).
    pub fn minor(&self) -> Vec2 {
        let major = self.major();
        Vec2::new(-major.y, major.x)
    }

    /// Blend two tensors with weights.
    pub fn blend(a: &Tensor, b: &Tensor, weight_a: f32, weight_b: f32) -> Self {
        let total = weight_a + weight_b;
        if total < 0.0001 {
            return Tensor::new(0.0, 0.0);
        }
        Self {
            r: (a.r * weight_a + b.r * weight_b) / total,
            s: (a.s * weight_a + b.s * weight_b) / total,
        }
    }
}

/// Types of basis fields that compose the tensor field.
#[derive(Clone, Debug)]
pub enum BasisField {
    /// Uniform grid aligned to global axes.
    Grid { angle: f32 },
    /// Radial field emanating from a center point.
    Radial { center: Vec2 },
    /// Field aligned to a polyline (river, highway).
    Polyline { points: Vec<Vec2> },
}

impl BasisField {
    /// Sample the basis field at a point.
    pub fn sample(&self, pos: Vec2) -> Tensor {
        match self {
            BasisField::Grid { angle } => Tensor::from_angle(*angle),
            BasisField::Radial { center } => {
                let dir = pos - *center;
                if dir.length_squared() < 0.0001 {
                    Tensor::new(0.0, 0.0)
                } else {
                    Tensor::from_direction(dir.normalize())
                }
            }
            BasisField::Polyline { points } => {
                // Find closest segment and align to it
                let mut min_dist = f32::MAX;
                let mut closest_dir = Vec2::X;

                for window in points.windows(2) {
                    let (a, b) = (window[0], window[1]);
                    let (dist, dir) = Self::point_to_segment(pos, a, b);
                    if dist < min_dist {
                        min_dist = dist;
                        closest_dir = dir;
                    }
                }

                Tensor::from_direction(closest_dir)
            }
        }
    }

    /// Calculate distance from point to segment and segment direction.
    fn point_to_segment(p: Vec2, a: Vec2, b: Vec2) -> (f32, Vec2) {
        let ab = b - a;
        let ap = p - a;
        let t = (ap.dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        let closest = a + ab * t;
        let dist = p.distance(closest);
        let dir = ab.normalize_or_zero();
        (dist, dir)
    }

    /// Weight function based on distance (decay factor).
    pub fn weight(&self, pos: Vec2, decay: f32) -> f32 {
        match self {
            BasisField::Grid { .. } => 1.0, // Global influence
            BasisField::Radial { center } => {
                let dist = pos.distance(*center);
                (-dist * decay).exp()
            }
            BasisField::Polyline { points } => {
                let mut min_dist = f32::MAX;
                for window in points.windows(2) {
                    let (a, b) = (window[0], window[1]);
                    let (dist, _) = Self::point_to_segment(pos, a, b);
                    min_dist = min_dist.min(dist);
                }
                (-min_dist * decay).exp()
            }
        }
    }
}

/// The composite tensor field resource.
#[derive(Resource, Default)]
pub struct TensorField {
    pub basis_fields: Vec<(BasisField, f32)>, // (field, decay_rate)
}

impl TensorField {
    /// Sample the composite field at a position.
    pub fn sample(&self, pos: Vec2) -> Tensor {
        if self.basis_fields.is_empty() {
            return Tensor::from_angle(0.0);
        }

        let mut result = Tensor::new(0.0, 0.0);
        let mut total_weight = 0.0;

        for (field, decay) in &self.basis_fields {
            let weight = field.weight(pos, *decay);
            let sample = field.sample(pos);
            result = Tensor::blend(&result, &sample, total_weight, weight);
            total_weight += weight;
        }

        result
    }

    /// Add a grid basis field.
    pub fn add_grid(&mut self, angle: f32) {
        self.basis_fields
            .push((BasisField::Grid { angle }, 0.0));
    }

    /// Add a radial basis field.
    pub fn add_radial(&mut self, center: Vec2, decay: f32) {
        self.basis_fields
            .push((BasisField::Radial { center }, decay));
    }

    /// Add a polyline basis field.
    pub fn add_polyline(&mut self, points: Vec<Vec2>, decay: f32) {
        self.basis_fields
            .push((BasisField::Polyline { points }, decay));
    }
}

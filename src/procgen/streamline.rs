//! Streamline integration for tracing roads through the tensor field.
//!
//! Uses a "turtle" approach to walk through the field following eigenvectors.

#![allow(dead_code)]

use bevy::prelude::*;

use super::tensor::TensorField;

/// Configuration for streamline integration.
#[derive(Clone, Debug)]
pub struct StreamlineConfig {
    /// Step size for integration.
    pub step_size: f32,
    /// Maximum steps per streamline.
    pub max_steps: usize,
    /// Minimum distance between streamlines.
    pub separation: f32,
    /// Snapping distance to existing nodes.
    pub snap_distance: f32,
}

impl Default for StreamlineConfig {
    fn default() -> Self {
        Self {
            step_size: 1.0,
            max_steps: 500,
            separation: 5.0,
            snap_distance: 2.0,
        }
    }
}

/// A point along a streamline.
#[derive(Clone, Copy, Debug)]
pub struct StreamlinePoint {
    pub position: Vec2,
    pub direction: Vec2,
}

/// A complete streamline (sequence of points).
#[derive(Clone, Debug)]
pub struct Streamline {
    pub points: Vec<StreamlinePoint>,
    pub is_major: bool, // true = major eigenvector, false = minor
}

/// Integrator for generating streamlines from the tensor field.
pub struct StreamlineIntegrator<'a> {
    field: &'a TensorField,
    config: StreamlineConfig,
}

impl<'a> StreamlineIntegrator<'a> {
    pub fn new(field: &'a TensorField, config: StreamlineConfig) -> Self {
        Self { field, config }
    }

    /// Trace a streamline from a seed point in both directions.
    pub fn trace(&self, seed: Vec2, use_major: bool) -> Streamline {
        let mut forward = self.trace_direction(seed, use_major, 1.0);
        let backward = self.trace_direction(seed, use_major, -1.0);

        // Combine: reverse backward, skip duplicate seed, append forward
        let mut points: Vec<StreamlinePoint> = backward.into_iter().rev().collect();
        if !forward.is_empty() {
            forward.remove(0); // Remove duplicate seed
        }
        points.extend(forward);

        Streamline {
            points,
            is_major: use_major,
        }
    }

    /// Trace in one direction from seed.
    fn trace_direction(&self, seed: Vec2, use_major: bool, sign: f32) -> Vec<StreamlinePoint> {
        let mut points = Vec::new();
        let mut pos = seed;
        let mut prev_dir = Vec2::ZERO;

        for _ in 0..self.config.max_steps {
            let tensor = self.field.sample(pos);
            let mut dir = if use_major {
                tensor.major()
            } else {
                tensor.minor()
            };

            // Ensure consistent direction (avoid 180 degree flips)
            if prev_dir != Vec2::ZERO && dir.dot(prev_dir) < 0.0 {
                dir = -dir;
            }
            dir *= sign;

            points.push(StreamlinePoint {
                position: pos,
                direction: dir,
            });

            // RK4 integration step
            let new_pos = self.rk4_step(pos, use_major, sign);

            // Check for degenerate tensor
            if (new_pos - pos).length() < 0.001 {
                break;
            }

            prev_dir = dir;
            pos = new_pos;
        }

        points
    }

    /// Single RK4 integration step.
    fn rk4_step(&self, pos: Vec2, use_major: bool, sign: f32) -> Vec2 {
        let h = self.config.step_size;

        let get_dir = |p: Vec2| -> Vec2 {
            let t = self.field.sample(p);
            let d = if use_major { t.major() } else { t.minor() };
            d * sign
        };

        let k1 = get_dir(pos);
        let k2 = get_dir(pos + k1 * h * 0.5);
        let k3 = get_dir(pos + k2 * h * 0.5);
        let k4 = get_dir(pos + k3 * h);

        pos + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (h / 6.0)
    }
}

/// Generate a grid of seed points for streamline tracing.
pub fn generate_seeds(bounds: Rect, spacing: f32) -> Vec<Vec2> {
    let mut seeds = Vec::new();
    let mut y = bounds.min.y;

    while y <= bounds.max.y {
        let mut x = bounds.min.x;
        while x <= bounds.max.x {
            seeds.push(Vec2::new(x, y));
            x += spacing;
        }
        y += spacing;
    }

    seeds
}

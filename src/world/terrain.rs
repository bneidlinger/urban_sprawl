//! Terrain generation and height maps.

#![allow(dead_code)]

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

/// Terrain height map.
#[derive(Resource)]
pub struct HeightMap {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
    pub scale: f32,
}

impl HeightMap {
    /// Generate terrain using Perlin noise.
    pub fn generate(width: usize, height: usize, seed: u32, scale: f32) -> Self {
        let perlin = Perlin::new(seed);
        let mut data = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 4.0;
                let ny = y as f64 / height as f64 * 4.0;

                // Octave noise for more natural terrain.
                let mut value = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;

                for _ in 0..4 {
                    value += perlin.get([nx * frequency, ny * frequency]) * amplitude;
                    amplitude *= 0.5;
                    frequency *= 2.0;
                }

                // Normalize to 0-1 range.
                let normalized = (value + 1.0) / 2.0;
                data.push(normalized as f32 * scale);
            }
        }

        Self {
            width,
            height,
            data,
            scale,
        }
    }

    /// Sample height at a position (bilinear interpolation).
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let fx = x.clamp(0.0, (self.width - 1) as f32);
        let fy = y.clamp(0.0, (self.height - 1) as f32);

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let tx = fx - fx.floor();
        let ty = fy - fy.floor();

        let h00 = self.data[y0 * self.width + x0];
        let h10 = self.data[y0 * self.width + x1];
        let h01 = self.data[y1 * self.width + x0];
        let h11 = self.data[y1 * self.width + x1];

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;

        h0 * (1.0 - ty) + h1 * ty
    }
}

/// Water body definition.
#[derive(Clone, Debug)]
pub struct WaterBody {
    pub boundary: Vec<Vec2>,
    pub water_level: f32,
}

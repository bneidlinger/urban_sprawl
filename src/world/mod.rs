//! World management: terrain, chunks, spatial partitioning.

#![allow(dead_code)]

use bevy::prelude::*;

pub mod grid;
pub mod terrain;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldConfig>();
    }
}

/// Global world configuration.
#[derive(Resource)]
pub struct WorldConfig {
    /// World size in meters.
    pub size: Vec2,
    /// Chunk size for spatial partitioning.
    pub chunk_size: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(2000.0, 2000.0),
            chunk_size: 100.0,
        }
    }
}

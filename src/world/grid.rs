//! Spatial partitioning grid for efficient queries.

use bevy::prelude::*;
use std::collections::HashMap;

/// Spatial hash grid for entity lookups.
#[derive(Resource, Default)]
pub struct SpatialGrid {
    pub cell_size: f32,
    pub cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Convert world position to cell coordinates.
    pub fn to_cell(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
        )
    }

    /// Insert an entity at a position.
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let cell = self.to_cell(pos);
        self.cells.entry(cell).or_default().push(entity);
    }

    /// Remove an entity from a position.
    pub fn remove(&mut self, entity: Entity, pos: Vec2) {
        let cell = self.to_cell(pos);
        if let Some(entities) = self.cells.get_mut(&cell) {
            entities.retain(|&e| e != entity);
        }
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Query entities in a radius.
    pub fn query_radius(&self, center: Vec2, radius: f32) -> Vec<Entity> {
        let min_cell = self.to_cell(center - Vec2::splat(radius));
        let max_cell = self.to_cell(center + Vec2::splat(radius));

        let mut result = Vec::new();

        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                if let Some(entities) = self.cells.get(&(cx, cy)) {
                    result.extend(entities);
                }
            }
        }

        result
    }

    /// Query entities in a cell.
    pub fn query_cell(&self, cell: (i32, i32)) -> &[Entity] {
        self.cells.get(&cell).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

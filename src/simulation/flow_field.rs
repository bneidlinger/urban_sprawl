//! Flow field (Dijkstra map) for agent navigation.
//!
//! All agents heading to the same destination share a single flow field.

#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

/// A flow field resource for a specific destination.
#[derive(Clone)]
pub struct FlowField {
    /// Width of the field grid.
    pub width: usize,
    /// Height of the field grid.
    pub height: usize,
    /// Distance values (cost to destination).
    pub distances: Vec<f32>,
    /// Direction vectors (negative gradient of distance).
    pub directions: Vec<Vec2>,
    /// Grid cell size in world units.
    pub cell_size: f32,
    /// World-space origin of the grid.
    pub origin: Vec2,
}

impl FlowField {
    pub fn new(width: usize, height: usize, cell_size: f32, origin: Vec2) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            distances: vec![f32::MAX; size],
            directions: vec![Vec2::ZERO; size],
            cell_size,
            origin,
        }
    }

    /// Convert world position to grid coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> Option<(usize, usize)> {
        let local = world_pos - self.origin;
        let gx = (local.x / self.cell_size).floor() as isize;
        let gy = (local.y / self.cell_size).floor() as isize;

        if gx >= 0 && gy >= 0 && (gx as usize) < self.width && (gy as usize) < self.height {
            Some((gx as usize, gy as usize))
        } else {
            None
        }
    }

    /// Get index from grid coordinates.
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Sample the flow direction at a world position.
    pub fn sample(&self, world_pos: Vec2) -> Vec2 {
        let Some((gx, gy)) = self.world_to_grid(world_pos) else {
            return Vec2::ZERO;
        };
        self.directions[self.index(gx, gy)]
    }

    /// Sample with bilinear interpolation.
    pub fn sample_smooth(&self, world_pos: Vec2) -> Vec2 {
        let local = world_pos - self.origin;
        let fx = local.x / self.cell_size;
        let fy = local.y / self.cell_size;

        let x0 = fx.floor() as isize;
        let y0 = fy.floor() as isize;

        let tx = fx - fx.floor();
        let ty = fy - fy.floor();

        let sample_at = |x: isize, y: isize| -> Vec2 {
            if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
                self.directions[self.index(x as usize, y as usize)]
            } else {
                Vec2::ZERO
            }
        };

        let d00 = sample_at(x0, y0);
        let d10 = sample_at(x0 + 1, y0);
        let d01 = sample_at(x0, y0 + 1);
        let d11 = sample_at(x0 + 1, y0 + 1);

        let d0 = d00.lerp(d10, tx);
        let d1 = d01.lerp(d11, tx);

        d0.lerp(d1, ty).normalize_or_zero()
    }
}

/// Node for Dijkstra priority queue.
#[derive(Clone, Copy)]
struct DijkstraNode {
    x: usize,
    y: usize,
    cost: f32,
}

impl Eq for DijkstraNode {}

impl PartialEq for DijkstraNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Cost function for movement (can incorporate terrain, roads, etc.).
pub type CostFn = fn(usize, usize) -> f32;

/// Generate a flow field using Dijkstra's algorithm.
pub fn generate_flow_field(
    width: usize,
    height: usize,
    cell_size: f32,
    origin: Vec2,
    goals: &[(usize, usize)],
    cost_fn: Option<CostFn>,
) -> FlowField {
    let mut field = FlowField::new(width, height, cell_size, origin);
    let mut heap = BinaryHeap::new();

    // Initialize goals with zero cost.
    for &(gx, gy) in goals {
        let idx = field.index(gx, gy);
        field.distances[idx] = 0.0;
        heap.push(DijkstraNode {
            x: gx,
            y: gy,
            cost: 0.0,
        });
    }

    let default_cost: CostFn = |_, _| 1.0;
    let cost = cost_fn.unwrap_or(default_cost);

    // Dijkstra expansion.
    while let Some(current) = heap.pop() {
        let idx = field.index(current.x, current.y);
        if current.cost > field.distances[idx] {
            continue;
        }

        // 8-connected neighbors.
        let neighbors: [(isize, isize); 8] = [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ];

        for (dx, dy) in neighbors {
            let nx = current.x as isize + dx;
            let ny = current.y as isize + dy;

            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                continue;
            }

            let nx = nx as usize;
            let ny = ny as usize;

            // Diagonal movement costs sqrt(2).
            let move_cost = if dx != 0 && dy != 0 {
                1.414 * cost(nx, ny)
            } else {
                cost(nx, ny)
            };

            let new_cost = current.cost + move_cost;
            let neighbor_idx = field.index(nx, ny);

            if new_cost < field.distances[neighbor_idx] {
                field.distances[neighbor_idx] = new_cost;
                heap.push(DijkstraNode {
                    x: nx,
                    y: ny,
                    cost: new_cost,
                });
            }
        }
    }

    // Compute direction vectors (negative gradient).
    for y in 0..height {
        for x in 0..width {
            let idx = field.index(x, y);
            let current_dist = field.distances[idx];

            if current_dist == f32::MAX {
                continue;
            }

            let mut best_dir = Vec2::ZERO;
            let mut best_drop = 0.0f32;

            let neighbors: [(isize, isize); 8] = [
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ];

            for (dx, dy) in neighbors {
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                    continue;
                }

                let neighbor_idx = field.index(nx as usize, ny as usize);
                let neighbor_dist = field.distances[neighbor_idx];
                let drop = current_dist - neighbor_dist;

                if drop > best_drop {
                    best_drop = drop;
                    best_dir = Vec2::new(dx as f32, dy as f32).normalize();
                }
            }

            field.directions[idx] = best_dir;
        }
    }

    field
}

/// Cache of flow fields keyed by destination entity.
#[derive(Resource, Default)]
pub struct FlowFieldCache {
    pub fields: HashMap<Entity, FlowField>,
}

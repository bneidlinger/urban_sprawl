//! Traffic simulation using Cellular Automata (Nagel-Schreckenberg model).
//!
//! Reference: Nagel, K., & Schreckenberg, M. (1992).
//! "A cellular automaton model for freeway traffic"

#![allow(dead_code)]

use bevy::prelude::*;
use rand::Rng;

/// Configuration for CA traffic simulation.
#[derive(Resource)]
pub struct TrafficConfig {
    /// Cell size in meters.
    pub cell_size: f32,
    /// Maximum velocity in cells/tick.
    pub max_velocity: u8,
    /// Probability of random slowdown.
    pub slowdown_prob: f32,
}

impl Default for TrafficConfig {
    fn default() -> Self {
        Self {
            cell_size: 7.5, // Typical car length + gap
            max_velocity: 5,
            slowdown_prob: 0.3,
        }
    }
}

/// A lane represented as a cellular automaton.
#[derive(Clone, Debug)]
pub struct CaLane {
    /// Cells: None = empty, Some(velocity) = occupied.
    pub cells: Vec<Option<u8>>,
    /// Length of the lane in cells.
    pub length: usize,
}

impl CaLane {
    pub fn new(length: usize) -> Self {
        Self {
            cells: vec![None; length],
            length,
        }
    }

    /// Calculate gap to next vehicle (cells until occupied).
    pub fn gap_ahead(&self, position: usize) -> usize {
        for offset in 1..self.length {
            let check_pos = (position + offset) % self.length;
            if self.cells[check_pos].is_some() {
                return offset - 1;
            }
        }
        self.length - 1
    }

    /// Single CA update step (Nagel-Schreckenberg rules).
    pub fn step(&mut self, config: &TrafficConfig, rng: &mut impl Rng) {
        let mut new_cells = vec![None; self.length];

        for (pos, cell) in self.cells.iter().enumerate() {
            let Some(mut velocity) = *cell else {
                continue;
            };

            // Rule 1: Acceleration
            if velocity < config.max_velocity {
                velocity += 1;
            }

            // Rule 2: Slowing down (gap)
            let gap = self.gap_ahead(pos) as u8;
            if velocity > gap {
                velocity = gap;
            }

            // Rule 3: Randomization
            if velocity > 0 && rng.gen::<f32>() < config.slowdown_prob {
                velocity -= 1;
            }

            // Rule 4: Movement
            let new_pos = (pos + velocity as usize) % self.length;
            new_cells[new_pos] = Some(velocity);
        }

        self.cells = new_cells;
    }

    /// Spawn a vehicle at position if empty.
    pub fn spawn(&mut self, position: usize, velocity: u8) -> bool {
        if position >= self.length || self.cells[position].is_some() {
            return false;
        }
        self.cells[position] = Some(velocity);
        true
    }

    /// Remove vehicle at position.
    pub fn despawn(&mut self, position: usize) -> Option<u8> {
        if position >= self.length {
            return None;
        }
        self.cells[position].take()
    }

    /// Count vehicles in lane.
    pub fn vehicle_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }

    /// Calculate current density (vehicles per cell).
    pub fn density(&self) -> f32 {
        self.vehicle_count() as f32 / self.length as f32
    }

    /// Calculate average velocity.
    pub fn average_velocity(&self) -> f32 {
        let total: u32 = self
            .cells
            .iter()
            .filter_map(|c| c.map(|v| v as u32))
            .sum();
        let count = self.vehicle_count();
        if count == 0 {
            0.0
        } else {
            total as f32 / count as f32
        }
    }
}

/// Road segment with CA lanes.
#[derive(Component)]
pub struct CaRoadSegment {
    pub forward_lanes: Vec<CaLane>,
    pub backward_lanes: Vec<CaLane>,
}

impl CaRoadSegment {
    pub fn new(length: usize, forward_count: usize, backward_count: usize) -> Self {
        Self {
            forward_lanes: (0..forward_count).map(|_| CaLane::new(length)).collect(),
            backward_lanes: (0..backward_count).map(|_| CaLane::new(length)).collect(),
        }
    }
}

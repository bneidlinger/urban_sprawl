//! Vehicle movement and lane logic.

use bevy::prelude::*;

/// Vehicle component.
#[derive(Component)]
pub struct Vehicle {
    /// Current road segment entity.
    pub road_segment: Entity,
    /// Lane index (0 = rightmost).
    pub lane: u8,
    /// Position along the segment (0.0 - 1.0).
    pub position: f32,
    /// Current velocity (units/sec).
    pub velocity: f32,
    /// Maximum velocity.
    pub max_velocity: f32,
    /// Acceleration rate.
    pub acceleration: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            road_segment: Entity::PLACEHOLDER,
            lane: 0,
            position: 0.0,
            velocity: 0.0,
            max_velocity: 15.0, // ~54 km/h
            acceleration: 3.0,
        }
    }
}

/// Route that a vehicle follows.
#[derive(Component)]
pub struct VehicleRoute {
    /// Sequence of road segment entities.
    pub segments: Vec<Entity>,
    /// Current index in the route.
    pub current_index: usize,
    /// Final destination entity.
    pub destination: Entity,
}

/// Marker for vehicles that are waiting at an intersection.
#[derive(Component)]
pub struct WaitingAtIntersection {
    pub intersection: Entity,
    pub wait_time: f32,
}

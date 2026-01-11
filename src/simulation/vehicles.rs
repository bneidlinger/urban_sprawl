//! Vehicle movement and lane logic.

#![allow(dead_code)]

use bevy::prelude::*;
use petgraph::graph::{EdgeIndex, NodeIndex};

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

/// Marker component for moving vehicles (distinct from ParkedCar).
#[derive(Component)]
pub struct MovingVehicle;

/// Navigation state for a vehicle traveling on the road network.
#[derive(Component)]
pub struct VehicleNavigation {
    /// Current edge being traversed.
    pub current_edge: EdgeIndex,
    /// Direction of travel (true = from node_a to node_b, false = reverse).
    pub forward: bool,
    /// Progress along current edge (0.0 to 1.0).
    pub progress: f32,
    /// Current speed (world units per second).
    pub speed: f32,
    /// Target speed for this vehicle.
    pub target_speed: f32,
    /// Node index we're heading toward.
    pub destination_node: NodeIndex,
    /// Node we came from (to avoid immediate U-turns).
    pub previous_node: Option<NodeIndex>,
    /// Whether the vehicle should be stopping (for traffic lights).
    pub stopping: bool,
    /// Current lane offset from road center (positive = right, negative = left).
    pub lane_offset: f32,
    /// Target lane offset (for smooth lane changes).
    pub target_lane_offset: f32,
}

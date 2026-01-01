//! Commute and traffic simulation.
//!
//! Calculates:
//! - Average commute times from residential to job locations
//! - Traffic congestion on roads
//! - Affects happiness and zone desirability

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::procgen::building_factory::BuildingArchetype;
use crate::procgen::roads::RoadGraph;
use crate::render::building_spawner::Building;

pub struct CommutePlugin;

impl Plugin for CommutePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommuteStats>()
            .init_resource::<TrafficState>()
            .add_systems(
                Update,
                (calculate_commute_stats, update_traffic_congestion)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// City-wide commute statistics.
#[derive(Resource, Default)]
pub struct CommuteStats {
    /// Average commute distance in world units.
    pub average_distance: f32,
    /// Worst case commute distance.
    pub max_distance: f32,
    /// Percentage of residents with jobs within acceptable distance.
    pub job_accessibility: f32,
    /// Commute quality score (0-100, higher is better).
    pub commute_score: f32,
    /// Timer for recalculation.
    update_timer: f32,
}

impl CommuteStats {
    /// Maximum acceptable commute distance.
    const ACCEPTABLE_COMMUTE: f32 = 150.0;
    /// Ideal commute distance for max score.
    const IDEAL_COMMUTE: f32 = 50.0;
}

/// Traffic congestion state.
#[derive(Resource)]
pub struct TrafficState {
    /// Overall congestion level (0.0 = free flow, 1.0 = gridlock).
    pub congestion: f32,
    /// Traffic volume (vehicles per update).
    pub volume: u32,
    /// Road capacity utilization percentage.
    pub utilization: f32,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            congestion: 0.0,
            volume: 0,
            utilization: 0.0,
        }
    }
}

fn calculate_commute_stats(
    time: Res<Time>,
    mut stats: ResMut<CommuteStats>,
    buildings: Query<(&Building, &GlobalTransform)>,
) {
    stats.update_timer += time.delta_secs();

    // Only recalculate every 3 seconds
    if stats.update_timer < 3.0 {
        return;
    }
    stats.update_timer = 0.0;

    // Collect residential and job locations
    let mut residential_positions: Vec<Vec2> = Vec::new();
    let mut job_positions: Vec<Vec2> = Vec::new();

    for (building, transform) in &buildings {
        let pos = Vec2::new(transform.translation().x, transform.translation().z);

        match building.building_type {
            BuildingArchetype::Residential => {
                residential_positions.push(pos);
            }
            BuildingArchetype::Commercial | BuildingArchetype::Industrial => {
                job_positions.push(pos);
            }
        }
    }

    if residential_positions.is_empty() || job_positions.is_empty() {
        stats.average_distance = 0.0;
        stats.max_distance = 0.0;
        stats.job_accessibility = 100.0;
        stats.commute_score = 100.0;
        return;
    }

    // Calculate commute distances (each residential building to nearest job)
    let mut total_distance = 0.0f32;
    let mut max_distance = 0.0f32;
    let mut accessible_count = 0u32;

    for res_pos in &residential_positions {
        // Find nearest job
        let mut min_dist = f32::MAX;
        for job_pos in &job_positions {
            let dist = res_pos.distance(*job_pos);
            min_dist = min_dist.min(dist);
        }

        total_distance += min_dist;
        max_distance = max_distance.max(min_dist);

        if min_dist <= CommuteStats::ACCEPTABLE_COMMUTE {
            accessible_count += 1;
        }
    }

    let count = residential_positions.len() as f32;
    stats.average_distance = total_distance / count;
    stats.max_distance = max_distance;
    stats.job_accessibility = (accessible_count as f32 / count * 100.0).min(100.0);

    // Calculate commute score
    // Perfect score if average commute is at or below ideal
    // Score decreases as commute increases
    let commute_ratio = stats.average_distance / CommuteStats::IDEAL_COMMUTE;
    stats.commute_score = if commute_ratio <= 1.0 {
        100.0
    } else {
        (100.0 / commute_ratio).clamp(0.0, 100.0)
    };
}

fn update_traffic_congestion(
    road_graph: Res<RoadGraph>,
    buildings: Query<&Building>,
    mut traffic: ResMut<TrafficState>,
) {
    // Estimate traffic based on number of buildings and road capacity
    let building_count = buildings.iter().count() as f32;
    let road_segments = road_graph.edge_count() as f32;

    if road_segments == 0.0 {
        traffic.congestion = 0.0;
        traffic.volume = 0;
        traffic.utilization = 0.0;
        return;
    }

    // Each building generates some traffic
    // Residential: outbound morning, inbound evening
    // Commercial/Industrial: inbound morning, outbound evening
    let estimated_trips = building_count * 2.0; // Round trips

    // Each road segment can handle ~50 vehicles efficiently
    let road_capacity = road_segments * 50.0;

    traffic.volume = estimated_trips as u32;
    traffic.utilization = (estimated_trips / road_capacity * 100.0).min(100.0);

    // Congestion increases non-linearly as utilization approaches capacity
    traffic.congestion = if traffic.utilization < 50.0 {
        0.0
    } else if traffic.utilization < 80.0 {
        (traffic.utilization - 50.0) / 30.0 * 0.3
    } else if traffic.utilization < 100.0 {
        0.3 + (traffic.utilization - 80.0) / 20.0 * 0.4
    } else {
        0.7 + ((traffic.utilization - 100.0) / 50.0 * 0.3).min(0.3)
    };
}

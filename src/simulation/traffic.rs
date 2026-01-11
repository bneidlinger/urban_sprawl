//! Traffic simulation using Cellular Automata (Nagel-Schreckenberg model).
//!
//! Reference: Nagel, K., & Schreckenberg, M. (1992).
//! "A cellular automaton model for freeway traffic"
//!
//! Each road segment is discretized into cells, with vehicles moving
//! according to CA rules that naturally produce realistic traffic patterns
//! including spontaneous traffic jams.

use bevy::prelude::*;
use petgraph::graph::EdgeIndex;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;

use super::SimulationTick;

pub struct TrafficCaPlugin;

impl Plugin for TrafficCaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrafficConfig>()
            .init_resource::<TrafficCaState>()
            .init_resource::<TrafficCaStats>()
            .add_systems(
                Update,
                (
                    initialize_ca_roads.run_if(should_initialize_ca),
                    update_traffic_ca,
                    spawn_despawn_vehicles,
                    update_traffic_stats,
                ),
            );
    }
}

fn should_initialize_ca(
    road_mesh: Query<&RoadMeshGenerated>,
    state: Res<TrafficCaState>,
) -> bool {
    !road_mesh.is_empty() && !state.initialized
}

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

    /// Total vehicle count in all lanes.
    pub fn total_vehicles(&self) -> usize {
        self.forward_lanes.iter().map(|l| l.vehicle_count()).sum::<usize>()
            + self.backward_lanes.iter().map(|l| l.vehicle_count()).sum::<usize>()
    }

    /// Total capacity (all cells in all lanes).
    pub fn total_capacity(&self) -> usize {
        self.forward_lanes.iter().map(|l| l.length).sum::<usize>()
            + self.backward_lanes.iter().map(|l| l.length).sum::<usize>()
    }

    /// Average density across all lanes.
    pub fn average_density(&self) -> f32 {
        let cap = self.total_capacity();
        if cap == 0 {
            return 0.0;
        }
        self.total_vehicles() as f32 / cap as f32
    }
}

/// Global state for the CA traffic simulation.
#[derive(Resource, Default)]
pub struct TrafficCaState {
    /// Whether the CA roads have been initialized.
    pub initialized: bool,
    /// Map from road graph edge index to CA segment index.
    pub edge_to_segment: Vec<Option<usize>>,
    /// All CA road segments.
    pub segments: Vec<CaRoadSegment>,
    /// Edge indices corresponding to segments.
    pub segment_edges: Vec<EdgeIndex>,
    /// RNG for simulation.
    pub rng_seed: u64,
}

/// Traffic statistics from the CA simulation.
#[derive(Resource, Default)]
pub struct TrafficCaStats {
    pub total_vehicles: usize,
    pub total_capacity: usize,
    pub average_density: f32,
    pub average_velocity: f32,
    pub congested_segments: usize,
    pub free_flow_segments: usize,
}

/// Initialize CA lanes for all road segments.
fn initialize_ca_roads(
    road_graph: Res<RoadGraph>,
    config: Res<TrafficConfig>,
    mut state: ResMut<TrafficCaState>,
) {
    state.initialized = true;
    state.rng_seed = 54321;

    let mut rng = StdRng::seed_from_u64(state.rng_seed);

    // Create edge-to-segment mapping
    let edge_count = road_graph.edges().count();
    state.edge_to_segment = vec![None; edge_count];

    for (idx, edge) in road_graph.edges().enumerate() {
        // Calculate number of cells based on road length
        let cell_count = (edge.length / config.cell_size).ceil() as usize;
        if cell_count < 2 {
            continue; // Skip very short segments
        }

        // Determine lane count based on road type
        let (forward_lanes, backward_lanes) = match edge.road_type {
            RoadType::Highway => (3, 3),
            RoadType::Major => (2, 2),
            RoadType::Minor => (1, 1),
            RoadType::Alley => (1, 0), // One-way alleys
        };

        let segment = CaRoadSegment::new(cell_count, forward_lanes, backward_lanes);

        let segment_idx = state.segments.len();
        state.edge_to_segment[idx] = Some(segment_idx);
        state.segments.push(segment);
        state.segment_edges.push(EdgeIndex::new(idx));
    }

    // Spawn initial vehicles (sparse)
    let target_density = 0.05; // 5% initial density

    for segment in state.segments.iter_mut() {
        for lane in segment.forward_lanes.iter_mut() {
            let target_vehicles = (lane.length as f32 * target_density) as usize;
            for _ in 0..target_vehicles {
                let pos = rng.gen_range(0..lane.length);
                let vel = rng.gen_range(1..=config.max_velocity);
                lane.spawn(pos, vel);
            }
        }
        for lane in segment.backward_lanes.iter_mut() {
            let target_vehicles = (lane.length as f32 * target_density) as usize;
            for _ in 0..target_vehicles {
                let pos = rng.gen_range(0..lane.length);
                let vel = rng.gen_range(1..=config.max_velocity);
                lane.spawn(pos, vel);
            }
        }
    }

    info!(
        "Traffic CA initialized: {} segments, {} total cells",
        state.segments.len(),
        state.segments.iter().map(|s| s.total_capacity()).sum::<usize>()
    );
}

/// Update all CA lanes each simulation tick.
fn update_traffic_ca(
    config: Res<TrafficConfig>,
    mut state: ResMut<TrafficCaState>,
    mut tick_events: EventReader<SimulationTick>,
) {
    // Only update on simulation ticks
    let tick_count = tick_events.read().count();
    if tick_count == 0 || !state.initialized {
        return;
    }

    let mut rng = StdRng::seed_from_u64(state.rng_seed.wrapping_add(tick_count as u64));
    state.rng_seed = state.rng_seed.wrapping_add(tick_count as u64);

    // Update each segment
    for segment in state.segments.iter_mut() {
        for lane in segment.forward_lanes.iter_mut() {
            lane.step(&config, &mut rng);
        }
        for lane in segment.backward_lanes.iter_mut() {
            lane.step(&config, &mut rng);
        }
    }
}

/// Spawn and despawn vehicles at segment boundaries.
fn spawn_despawn_vehicles(
    config: Res<TrafficConfig>,
    mut state: ResMut<TrafficCaState>,
    mut tick_events: EventReader<SimulationTick>,
) {
    let tick_count = tick_events.read().count();
    if tick_count == 0 || !state.initialized {
        return;
    }

    let mut rng = StdRng::seed_from_u64(state.rng_seed.wrapping_add(1000));

    // Target density to maintain
    let target_density = 0.08;
    let spawn_chance = 0.1;
    let despawn_chance = 0.05;

    for segment in state.segments.iter_mut() {
        let current_density = segment.average_density();

        // Spawn at entrances if below target
        if current_density < target_density && rng.gen::<f32>() < spawn_chance {
            for lane in segment.forward_lanes.iter_mut() {
                if lane.cells[0].is_none() {
                    lane.spawn(0, rng.gen_range(1..=config.max_velocity));
                    break;
                }
            }
        }

        // Also spawn in backward lanes
        if current_density < target_density && rng.gen::<f32>() < spawn_chance {
            for lane in segment.backward_lanes.iter_mut() {
                if lane.cells[0].is_none() {
                    lane.spawn(0, rng.gen_range(1..=config.max_velocity));
                    break;
                }
            }
        }

        // Despawn at exits
        for lane in segment.forward_lanes.iter_mut() {
            let last_idx = lane.length.saturating_sub(1);
            if lane.cells[last_idx].is_some() && rng.gen::<f32>() < despawn_chance {
                lane.despawn(last_idx);
            }
        }

        for lane in segment.backward_lanes.iter_mut() {
            let last_idx = lane.length.saturating_sub(1);
            if lane.cells[last_idx].is_some() && rng.gen::<f32>() < despawn_chance {
                lane.despawn(last_idx);
            }
        }
    }
}

/// Update traffic statistics.
fn update_traffic_stats(
    state: Res<TrafficCaState>,
    mut stats: ResMut<TrafficCaStats>,
) {
    if !state.initialized || state.segments.is_empty() {
        return;
    }

    let mut total_vehicles = 0usize;
    let mut total_capacity = 0usize;
    let mut total_velocity = 0.0f32;
    let mut velocity_samples = 0usize;
    let mut congested = 0usize;
    let mut free_flow = 0usize;

    for segment in state.segments.iter() {
        total_vehicles += segment.total_vehicles();
        total_capacity += segment.total_capacity();

        let density = segment.average_density();

        if density > 0.3 {
            congested += 1;
        } else if density < 0.1 {
            free_flow += 1;
        }

        for lane in segment.forward_lanes.iter() {
            let vel = lane.average_velocity();
            if vel > 0.0 {
                total_velocity += vel;
                velocity_samples += 1;
            }
        }
        for lane in segment.backward_lanes.iter() {
            let vel = lane.average_velocity();
            if vel > 0.0 {
                total_velocity += vel;
                velocity_samples += 1;
            }
        }
    }

    stats.total_vehicles = total_vehicles;
    stats.total_capacity = total_capacity;
    stats.average_density = if total_capacity > 0 {
        total_vehicles as f32 / total_capacity as f32
    } else {
        0.0
    };
    stats.average_velocity = if velocity_samples > 0 {
        total_velocity / velocity_samples as f32
    } else {
        0.0
    };
    stats.congested_segments = congested;
    stats.free_flow_segments = free_flow;
}

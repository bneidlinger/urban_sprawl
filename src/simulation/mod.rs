//! Simulation systems for citizens, vehicles, and traffic.

use bevy::prelude::*;

pub mod citizens;
pub mod flow_field;
pub mod traffic;
pub mod vehicles;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationConfig>()
            .add_systems(Update, simulation_tick);
    }
}

/// Configuration for the simulation.
#[derive(Resource)]
pub struct SimulationConfig {
    /// Ticks per second for simulation updates.
    pub tick_rate: f32,
    /// Current simulation speed multiplier.
    pub speed: f32,
    /// Whether simulation is paused.
    pub paused: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            tick_rate: 20.0,
            speed: 1.0,
            paused: false,
        }
    }
}

/// Accumulator for fixed timestep simulation.
#[derive(Resource, Default)]
pub struct SimulationAccumulator {
    pub accumulated: f32,
}

fn simulation_tick(
    config: Res<SimulationConfig>,
    mut accumulator: Local<f32>,
    time: Res<Time>,
) {
    if config.paused {
        return;
    }

    *accumulator += time.delta_secs() * config.speed;
    let tick_duration = 1.0 / config.tick_rate;

    while *accumulator >= tick_duration {
        *accumulator -= tick_duration;
        // TODO: Run simulation systems here
    }
}

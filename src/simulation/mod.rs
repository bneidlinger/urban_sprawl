//! Simulation systems for citizens, vehicles, traffic, economy, and city growth.
//!
//! The simulation runs on a fixed timestep (default 20 Hz) decoupled from rendering.
//! Systems can listen for `SimulationTick` events for synchronized updates.

use bevy::prelude::*;

pub mod bus_routes;
pub mod citizens;
pub mod commute;
pub mod demand;
pub mod economy;
pub mod flow_field;
pub mod land_value;
pub mod pedestrians;
pub mod population;
pub mod services;
pub mod traffic;
pub mod vehicle_traffic;
pub mod vehicles;
pub mod zones;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(vehicle_traffic::MovingVehiclePlugin)
            .add_plugins(bus_routes::BusRoutesPlugin)
            .add_plugins(pedestrians::PedestrianPlugin)
            .add_plugins(economy::EconomyPlugin)
            .add_plugins(demand::DemandPlugin)
            .add_plugins(population::PopulationPlugin)
            .add_plugins(zones::ZoneGrowthPlugin)
            .add_plugins(land_value::LandValuePlugin)
            .add_plugins(services::ServiceCoveragePlugin)
            .add_plugins(commute::CommutePlugin)
            .add_plugins(citizens::CitizensPlugin)
            .add_plugins(traffic::TrafficCaPlugin)
            .add_plugins(flow_field::FlowFieldPlugin)
            .init_resource::<SimulationConfig>()
            .init_resource::<SimulationStats>()
            .add_event::<SimulationTick>()
            .add_systems(Update, (simulation_tick_system, simulation_controls));
    }
}

/// Configuration for the simulation.
#[derive(Resource)]
pub struct SimulationConfig {
    /// Ticks per second for simulation updates.
    pub tick_rate: f32,
    /// Current simulation speed multiplier (0.5x to 4x).
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

/// Event sent each simulation tick (at tick_rate Hz).
#[derive(Event)]
pub struct SimulationTick {
    /// The tick number since simulation start.
    pub tick: u64,
    /// Delta time for this tick (1.0 / tick_rate).
    pub delta: f32,
}

/// Statistics about the simulation.
#[derive(Resource, Default)]
pub struct SimulationStats {
    /// Total ticks since simulation start.
    pub total_ticks: u64,
    /// Accumulated time for fixed timestep.
    pub accumulator: f32,
}

/// System that generates simulation ticks at fixed intervals.
fn simulation_tick_system(
    config: Res<SimulationConfig>,
    mut stats: ResMut<SimulationStats>,
    time: Res<Time>,
    mut tick_events: EventWriter<SimulationTick>,
) {
    if config.paused {
        return;
    }

    stats.accumulator += time.delta_secs() * config.speed;
    let tick_duration = 1.0 / config.tick_rate;

    // Process accumulated time, sending tick events
    while stats.accumulator >= tick_duration {
        stats.accumulator -= tick_duration;
        stats.total_ticks += 1;

        tick_events.send(SimulationTick {
            tick: stats.total_ticks,
            delta: tick_duration,
        });
    }
}

/// Keyboard controls for simulation speed and pause.
fn simulation_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<SimulationConfig>,
) {
    // Space: Toggle pause
    if keyboard.just_pressed(KeyCode::Space) {
        config.paused = !config.paused;
        if config.paused {
            info!("Simulation PAUSED");
        } else {
            info!("Simulation RESUMED ({}x speed)", config.speed);
        }
    }

    // Number keys for speed presets
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.speed = 1.0;
        info!("Simulation speed: 1x");
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        config.speed = 2.0;
        info!("Simulation speed: 2x");
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        config.speed = 3.0;
        info!("Simulation speed: 3x");
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        config.speed = 4.0;
        info!("Simulation speed: 4x");
    }

    // +/- for speed adjustment
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        config.speed = (config.speed + 0.5).min(4.0);
        info!("Simulation speed: {}x", config.speed);
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        config.speed = (config.speed - 0.5).max(0.5);
        info!("Simulation speed: {}x", config.speed);
    }
}

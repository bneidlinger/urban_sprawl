//! Core game state and mode management.
//!
//! Provides state machines for controlling game flow and distinguishing
//! between Sandbox (blank canvas) and Procedural (generated roads) modes.

use bevy::prelude::*;

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<GameMode>()
            .init_resource::<SimulationSpeed>();
    }
}

/// High-level game state controlling which systems run.
#[derive(States, Default, Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    /// Main menu is displayed, simulation paused.
    #[default]
    MainMenu,
    /// Loading/generating city data.
    Loading,
    /// Active gameplay - simulation running.
    Playing,
    /// Game paused (ESC menu, etc).
    Paused,
}

/// Distinguishes how the city was initialized.
#[derive(States, Default, Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum GameMode {
    /// No game started yet.
    #[default]
    None,
    /// Player builds from a blank canvas - no procedural roads.
    Sandbox,
    /// Procedurally generated road network to start.
    Procedural,
}

/// Controls simulation tick speed.
#[derive(Resource)]
pub struct SimulationSpeed {
    /// True if simulation is paused (time doesn't advance).
    pub paused: bool,
    /// Speed multiplier: 1.0 = normal, 2.0 = fast, 0.5 = slow.
    pub speed: f32,
}

impl Default for SimulationSpeed {
    fn default() -> Self {
        Self {
            paused: false,
            speed: 1.0,
        }
    }
}

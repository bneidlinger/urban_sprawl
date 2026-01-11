//! IsoCitySim - Large-scale isometric city simulator
//!
//! A Bevy-based city simulation targeting 100,000+ entities with
//! procedural generation using tensor fields and shape grammars.

use bevy::prelude::*;

mod audio;
mod camera;
mod game_state;
mod procgen;
mod render;
mod simulation;
mod tools;
mod ui;
mod world;

fn main() {
    // Force Vulkan backend on Windows (DX12 causes crashes on some systems)
    #[cfg(target_os = "windows")]
    std::env::set_var("WGPU_BACKEND", "vulkan");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "IsoCitySim".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        // Game state management
        .add_plugins(game_state::GameStatePlugin)
        // Player tools
        .add_plugins(tools::ToolsPlugin)
        // Core plugins
        .add_plugins(camera::CameraPlugin)
        .add_plugins(render::RenderPlugin)
        // Procedural generation
        .add_plugins(procgen::ProcgenPlugin)
        // Simulation
        .add_plugins(simulation::SimulationPlugin)
        // World management
        .add_plugins(world::WorldPlugin)
        // Debug UI
        .add_plugins(ui::UiPlugin)
        // Ambient audio
        .add_plugins(audio::AudioPlugin)
        .run();
}

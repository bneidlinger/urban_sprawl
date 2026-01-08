//! Debug UI and visualization tools.

use bevy::{
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::render::day_night::TimeOfDay;
use crate::render::gpu_culling::CullStats;
use crate::render::building_spawner::Building;
use crate::simulation::SimulationConfig;

pub mod debug_render;
pub mod menu;
pub mod stats_bar;
pub mod toolbox;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(menu::MenuPlugin)
            .add_plugins(toolbox::ToolboxPlugin)
            .add_plugins(stats_bar::StatsBarPlugin)
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .add_plugins(debug_render::DebugRenderPlugin)
            .init_resource::<DebugConfig>()
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    update_fps_counter,
                    update_frame_stats,
                    update_time_display,
                    update_sim_status,
                    handle_time_controls,
                    toggle_debug_views,
                ),
            );
    }
}

/// Configuration for debug visualization.
#[derive(Resource)]
pub struct DebugConfig {
    pub show_fps: bool,
    pub show_tensor_field: bool,
    pub show_road_graph: bool,
    pub show_flow_fields: bool,
    pub show_grid: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_fps: true,
            show_tensor_field: false,
            show_road_graph: false, // Disabled - using mesh rendering now
            show_flow_fields: false,
            show_grid: false,
        }
    }
}

/// Marker for the FPS text entity.
#[derive(Component)]
struct FpsText;

/// Marker for time display text.
#[derive(Component)]
struct TimeText;

/// Marker for debug info text.
#[derive(Component)]
struct DebugInfoText;

/// Marker for simulation status text.
#[derive(Component)]
struct SimStatusText;

/// Marker for frame stats text (entities, draw calls, culling).
#[derive(Component)]
struct FrameStatsText;

fn setup_hud(mut commands: Commands) {
    let panel_bg = Color::srgb(0.04, 0.05, 0.06);
    let border = Color::srgb(0.0, 0.75, 0.35);
    let retro_green = Color::srgb(0.4, 0.95, 0.6);
    let retro_orange = Color::srgb(1.0, 0.6, 0.2);

    // Top-right HUD stack (moved from left to avoid toolbox overlap)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(6.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(panel_bg),
            BorderColor(border),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("URBAN SPRAWL // SYS MONITOR"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(retro_orange),
            ));

            parent.spawn((
                Text::new("FPS: --"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(retro_green),
                FpsText,
            ));

            parent.spawn((
                Text::new("--:-- --"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(retro_green),
                TimeText,
            ));

            parent.spawn((
                Text::new("SIM: 1.0x | TIME: 0.5x"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.95, 0.8)),
                SimStatusText,
            ));

            parent.spawn((
                Text::new("[P] Pause | [ [ / ] ] Time | [1-4] Dawn/Day/Dusk/Night"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.7)),
                DebugInfoText,
            ));

            // Frame stats section
            parent.spawn((
                Text::new("--- RENDER STATS ---"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(retro_orange),
            ));

            parent.spawn((
                Text::new("Entities: -- | Meshes: -- | Culled: --"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.9, 0.7)),
                FrameStatsText,
            ));
        });

    // Bottom control reminder
    commands.spawn((
        Text::new("WASD: Pan | Scroll: Zoom | Q/E: Rotate | F: Flow | G: Grid"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.85, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
    config: Res<DebugConfig>,
) {
    if !config.show_fps {
        return;
    }

    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                **text = format!("FPS: {:.0}", value);
            }
        }
    }
}

/// Update frame stats display with entity counts, mesh counts, and culling info.
fn update_frame_stats(
    diagnostics: Res<DiagnosticsStore>,
    cull_stats: Res<CullStats>,
    mesh_query: Query<&Mesh3d>,
    building_query: Query<&Building>,
    mut query: Query<&mut Text, With<FrameStatsText>>,
) {
    // Get total entity count from diagnostics
    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as usize;

    // Count meshes (approximate draw calls)
    let mesh_count = mesh_query.iter().count();

    // Count buildings specifically
    let building_count = building_query.iter().count();

    // Culling stats
    let visible = cull_stats.visible_objects;
    let culled = cull_stats.culled_objects;
    let cull_pct = cull_stats.cull_ratio * 100.0;

    for mut text in &mut query {
        **text = format!(
            "Ent: {} | Mesh: {} | Bldg: {} | Vis: {} | Cull: {} ({:.0}%)",
            entity_count, mesh_count, building_count, visible, culled, cull_pct
        );
    }
}

fn update_time_display(
    tod: Res<TimeOfDay>,
    mut query: Query<(&mut Text, &mut TextColor), With<TimeText>>,
) {
    let hour = tod.hour();
    let hours = hour as u32;
    let minutes = ((hour.fract()) * 60.0) as u32;

    // Format as 12-hour time
    let (display_hour, ampm) = if hours == 0 {
        (12, "AM")
    } else if hours < 12 {
        (hours, "AM")
    } else if hours == 12 {
        (12, "PM")
    } else {
        (hours - 12, "PM")
    };

    // Time period indicator
    let period = if hour >= 5.0 && hour < 7.0 {
        "Dawn"
    } else if hour >= 7.0 && hour < 12.0 {
        "Morning"
    } else if hour >= 12.0 && hour < 17.0 {
        "Afternoon"
    } else if hour >= 17.0 && hour < 20.0 {
        "Evening"
    } else {
        "Night"
    };

    for (mut text, mut color) in &mut query {
        let pause_indicator = if tod.paused { " [PAUSED]" } else { "" };
        **text = format!(
            "{}:{:02} {} - {}{}",
            display_hour, minutes, ampm, period, pause_indicator
        );

        // Color based on time of day
        color.0 = if tod.is_night() {
            Color::srgb(0.7, 0.8, 1.0) // Cool blue at night
        } else if hour >= 5.0 && hour < 8.0 {
            Color::srgb(1.0, 0.8, 0.6) // Warm orange at dawn
        } else if hour >= 17.0 && hour < 20.0 {
            Color::srgb(1.0, 0.7, 0.5) // Warm orange at dusk
        } else {
            Color::WHITE // White during day
        };
    }
}

fn update_sim_status(
    config: Res<SimulationConfig>,
    tod: Res<TimeOfDay>,
    mut query: Query<&mut Text, With<SimStatusText>>,
) {
    if config.is_changed() || tod.is_changed() {
        let status = if config.paused || tod.paused {
            "PAUSED"
        } else {
            "LIVE"
        };
        let sim_speed = format!("{:.1}x", config.speed);
        let time_speed = format!("{:.2}x", tod.speed);

        for mut text in &mut query {
            **text = format!(
                "SIM: {} | GAME: {} | STATE: {}",
                sim_speed, time_speed, status
            );
        }
    }
}

fn handle_time_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut tod: ResMut<TimeOfDay>,
    mut sim: ResMut<SimulationConfig>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        tod.paused = !tod.paused;
        sim.paused = tod.paused;
    }

    // Speed control for time-of-day (shared with simulation speed for coherence)
    if keys.just_pressed(KeyCode::BracketLeft) {
        tod.speed = (tod.speed * 0.5).clamp(0.05, 8.0);
        sim.speed = (sim.speed * 0.5).clamp(0.1, 8.0);
    }

    if keys.just_pressed(KeyCode::BracketRight) {
        tod.speed = (tod.speed * 2.0).clamp(0.05, 8.0);
        sim.speed = (sim.speed * 2.0).clamp(0.1, 8.0);
    }

    // Time of day presets
    if keys.just_pressed(KeyCode::Digit1) {
        tod.time = 0.20; // Dawn
    }
    if keys.just_pressed(KeyCode::Digit2) {
        tod.time = 0.5; // Midday
    }
    if keys.just_pressed(KeyCode::Digit3) {
        tod.time = 0.75; // Dusk
    }
    if keys.just_pressed(KeyCode::Digit4) {
        tod.time = 0.95; // Late night
    }
}

/// Toggle debug visualization modes with keyboard.
fn toggle_debug_views(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<DebugConfig>) {
    if keys.just_pressed(KeyCode::KeyT) {
        config.show_tensor_field = !config.show_tensor_field;
        info!(
            "Tensor field: {}",
            if config.show_tensor_field {
                "ON"
            } else {
                "OFF"
            }
        );
    }

    if keys.just_pressed(KeyCode::KeyR) {
        config.show_road_graph = !config.show_road_graph;
        info!(
            "Road graph: {}",
            if config.show_road_graph { "ON" } else { "OFF" }
        );
    }

    if keys.just_pressed(KeyCode::KeyF) {
        config.show_flow_fields = !config.show_flow_fields;
        info!(
            "Flow fields: {}",
            if config.show_flow_fields { "ON" } else { "OFF" }
        );
    }

    if keys.just_pressed(KeyCode::KeyG) {
        config.show_grid = !config.show_grid;
        info!(
            "Grid overlay: {}",
            if config.show_grid { "ON" } else { "OFF" }
        );
    }
}

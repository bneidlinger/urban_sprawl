//! Debug UI and visualization tools.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::render::day_night::TimeOfDay;

pub mod debug_render;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(debug_render::DebugRenderPlugin)
            .init_resource::<DebugConfig>()
            .add_systems(Startup, setup_fps_counter)
            .add_systems(Update, (update_fps_counter, update_time_display, toggle_debug_views));
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

fn setup_fps_counter(mut commands: Commands) {
    // FPS counter in top-left
    commands.spawn((
        Text::new("FPS: --"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        FpsText,
    ));

    // Time display in top-right
    commands.spawn((
        Text::new("12:00"),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        TimeText,
    ));

    // Debug toggle info
    commands.spawn((
        Text::new("[T] Tensor  [R] Roads  [P] Pause Time  [1-4] Time Presets"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugInfoText,
    ));

    // Controls hint
    commands.spawn((
        Text::new("WASD: Pan | Scroll: Zoom | Q/E: Rotate | [/]: Time Speed"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
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
        **text = format!("{}:{:02} {} - {}{}", display_hour, minutes, ampm, period, pause_indicator);

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

/// Toggle debug visualization modes with keyboard.
fn toggle_debug_views(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<DebugConfig>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        config.show_tensor_field = !config.show_tensor_field;
        info!("Tensor field: {}", if config.show_tensor_field { "ON" } else { "OFF" });
    }

    if keys.just_pressed(KeyCode::KeyR) {
        config.show_road_graph = !config.show_road_graph;
        info!("Road graph: {}", if config.show_road_graph { "ON" } else { "OFF" });
    }
}

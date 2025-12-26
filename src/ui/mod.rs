//! Debug UI and visualization tools.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub mod debug_render;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(debug_render::DebugRenderPlugin)
            .init_resource::<DebugConfig>()
            .add_systems(Startup, setup_fps_counter)
            .add_systems(Update, (update_fps_counter, toggle_debug_views));
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

    // Debug toggle info
    commands.spawn((
        Text::new("[T] Tensor Field  [R] Roads"),
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
        Text::new("WASD: Pan | Scroll: Zoom | Q/E: Rotate"),
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

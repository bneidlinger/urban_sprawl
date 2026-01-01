//! Toolbox panel for selecting player tools.

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::tools::{ActiveTool, ServiceType, ZoneType};

pub struct ToolboxPlugin;

impl Plugin for ToolboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_toolbox)
            .add_systems(
                Update,
                (handle_tool_buttons, handle_keyboard_shortcuts, update_button_styles)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Root entity for the toolbox panel.
#[derive(Component)]
struct ToolboxRoot;

/// Marker for a tool selection button.
#[derive(Component)]
struct ToolButton(ActiveTool);

// UI Colors
const PANEL_BG: Color = Color::srgba(0.05, 0.07, 0.06, 0.9);
const BUTTON_IDLE: Color = Color::srgba(0.1, 0.12, 0.11, 0.95);
const BUTTON_HOVER: Color = Color::srgba(0.15, 0.18, 0.16, 0.95);
const BUTTON_SELECTED: Color = Color::srgba(0.2, 0.4, 0.3, 0.95);
const BORDER: Color = Color::srgb(0.0, 0.5, 0.3);
const TEXT_COLOR: Color = Color::srgb(0.8, 0.95, 0.85);

fn setup_toolbox(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            ToolboxRoot,
        ))
        .with_children(|panel| {
            // Header
            panel.spawn((
                Text::new("TOOLS"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));

            // Zone tools section
            panel.spawn((
                Text::new("Zones:"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.7, 0.65)),
            ));

            // Zone buttons
            spawn_tool_button(panel, &font, "R", ActiveTool::ZonePaint(ZoneType::Residential), Color::srgb(0.2, 0.8, 0.3));
            spawn_tool_button(panel, &font, "C", ActiveTool::ZonePaint(ZoneType::Commercial), Color::srgb(0.3, 0.5, 0.9));
            spawn_tool_button(panel, &font, "I", ActiveTool::ZonePaint(ZoneType::Industrial), Color::srgb(0.9, 0.7, 0.2));

            // Infrastructure section
            panel.spawn((
                Text::new("Roads:"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.7, 0.65)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));

            spawn_tool_button(panel, &font, "Rd", ActiveTool::RoadDraw, Color::srgb(0.5, 0.5, 0.6));

            // Services section
            panel.spawn((
                Text::new("Services:"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.7, 0.65)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));

            spawn_tool_button(panel, &font, "Po", ActiveTool::PlaceService(ServiceType::Police), Color::srgb(0.3, 0.4, 0.8));
            spawn_tool_button(panel, &font, "Fi", ActiveTool::PlaceService(ServiceType::Fire), Color::srgb(0.9, 0.3, 0.2));
            spawn_tool_button(panel, &font, "Ho", ActiveTool::PlaceService(ServiceType::Hospital), Color::srgb(0.9, 0.9, 0.9));
            spawn_tool_button(panel, &font, "Sc", ActiveTool::PlaceService(ServiceType::School), Color::srgb(0.9, 0.7, 0.2));
            spawn_tool_button(panel, &font, "Pk", ActiveTool::PlaceService(ServiceType::Park), Color::srgb(0.2, 0.7, 0.3));

            // Other tools
            panel.spawn((
                Text::new("Actions:"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.7, 0.65)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));

            spawn_tool_button(panel, &font, "X", ActiveTool::Demolish, Color::srgb(0.9, 0.3, 0.3));
            spawn_tool_button(panel, &font, "?", ActiveTool::Query, Color::srgb(0.5, 0.5, 0.5));
        });
}

fn spawn_tool_button(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    label: &str,
    tool: ActiveTool,
    color: Color,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            BorderColor(color),
            ToolButton(tool),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(color),
            ));
        });
}

fn handle_tool_buttons(
    interactions: Query<(&Interaction, &ToolButton), (Changed<Interaction>, With<Button>)>,
    mut next_tool: ResMut<NextState<ActiveTool>>,
) {
    for (interaction, ToolButton(tool)) in &interactions {
        if *interaction == Interaction::Pressed {
            next_tool.set(*tool);
            info!("Selected tool: {:?}", tool);
        }
    }
}

fn handle_keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_tool: ResMut<NextState<ActiveTool>>,
) {
    // Zone shortcuts (R/C/I for Residential/Commercial/Industrial)
    if keyboard.just_pressed(KeyCode::KeyR) {
        next_tool.set(ActiveTool::ZonePaint(ZoneType::Residential));
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        next_tool.set(ActiveTool::ZonePaint(ZoneType::Commercial));
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        next_tool.set(ActiveTool::ZonePaint(ZoneType::Industrial));
    }

    // Road drawing
    if keyboard.just_pressed(KeyCode::KeyD) {
        next_tool.set(ActiveTool::RoadDraw);
    }

    // Other tools
    if keyboard.just_pressed(KeyCode::KeyX) {
        next_tool.set(ActiveTool::Demolish);
    }
    // Note: Q conflicts with camera rotate, use V for Query/View
    if keyboard.just_pressed(KeyCode::KeyV) {
        next_tool.set(ActiveTool::Query);
    }

    // Escape to deselect
    if keyboard.just_pressed(KeyCode::Escape) {
        next_tool.set(ActiveTool::None);
    }
}

fn update_button_styles(
    current_tool: Res<State<ActiveTool>>,
    mut buttons: Query<(&ToolButton, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    for (ToolButton(tool), interaction, mut bg) in &mut buttons {
        let is_selected = *tool == *current_tool.get();

        bg.0 = match *interaction {
            Interaction::Pressed => BUTTON_SELECTED,
            Interaction::Hovered => {
                if is_selected {
                    BUTTON_SELECTED
                } else {
                    BUTTON_HOVER
                }
            }
            Interaction::None => {
                if is_selected {
                    BUTTON_SELECTED
                } else {
                    BUTTON_IDLE
                }
            }
        };
    }
}

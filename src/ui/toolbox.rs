//! Toolbox panel for selecting player tools.

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::procgen::roads::RoadType;
use crate::tools::road_draw::RoadDrawConfig;
use crate::tools::{ActiveTool, ServiceType, ZoneType};

pub struct ToolboxPlugin;

impl Plugin for ToolboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_toolbox)
            .add_systems(
                Update,
                (
                    handle_tool_buttons,
                    handle_road_type_buttons,
                    handle_keyboard_shortcuts,
                    update_button_styles,
                    update_road_type_button_styles,
                )
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

/// Marker for road type selection button.
#[derive(Component)]
struct RoadTypeButton(RoadType);

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

            // Infrastructure section - Road types
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

            // Road type buttons - each activates road draw with that type
            spawn_road_type_button(panel, &font, "Hw", RoadType::Highway, Color::srgb(0.7, 0.5, 0.3));
            spawn_road_type_button(panel, &font, "Mj", RoadType::Major, Color::srgb(0.5, 0.5, 0.6));
            spawn_road_type_button(panel, &font, "Mn", RoadType::Minor, Color::srgb(0.4, 0.45, 0.5));
            spawn_road_type_button(panel, &font, "Al", RoadType::Alley, Color::srgb(0.35, 0.38, 0.4));

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
    mut road_config: ResMut<RoadDrawConfig>,
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

    // Road type shortcuts (1-4) - activates road draw with that type
    if keyboard.just_pressed(KeyCode::Digit1) {
        road_config.road_type = RoadType::Highway;
        next_tool.set(ActiveTool::RoadDraw);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        road_config.road_type = RoadType::Major;
        next_tool.set(ActiveTool::RoadDraw);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        road_config.road_type = RoadType::Minor;
        next_tool.set(ActiveTool::RoadDraw);
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        road_config.road_type = RoadType::Alley;
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

fn spawn_road_type_button(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    label: &str,
    road_type: RoadType,
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
            RoadTypeButton(road_type),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(color),
            ));
        });
}

fn handle_road_type_buttons(
    interactions: Query<(&Interaction, &RoadTypeButton), (Changed<Interaction>, With<Button>)>,
    mut next_tool: ResMut<NextState<ActiveTool>>,
    mut road_config: ResMut<RoadDrawConfig>,
) {
    for (interaction, RoadTypeButton(road_type)) in &interactions {
        if *interaction == Interaction::Pressed {
            // Set road type and activate road drawing tool
            road_config.road_type = *road_type;
            next_tool.set(ActiveTool::RoadDraw);
            info!("Selected road type: {:?}", road_type);
        }
    }
}

fn update_road_type_button_styles(
    current_tool: Res<State<ActiveTool>>,
    road_config: Res<RoadDrawConfig>,
    mut buttons: Query<(&RoadTypeButton, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    let is_road_draw_active = matches!(current_tool.get(), ActiveTool::RoadDraw);

    for (RoadTypeButton(road_type), interaction, mut bg) in &mut buttons {
        // Button is selected if road draw is active AND this is the current road type
        let is_selected = is_road_draw_active && *road_type == road_config.road_type;

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

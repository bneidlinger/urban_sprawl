//! Main menu and terrain selection overlay.
//!
//! Menu flow:
//! 1. Mode selection (Sandbox vs Procedural)
//! 2. If Procedural: Terrain preset selection
//! 3. Start game

use bevy::prelude::*;

use crate::game_state::{GameMode, GameState};
use crate::render::day_night::TimeOfDay;
use crate::simulation::SimulationConfig;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .init_resource::<PreviousMenuPhase>()
            .add_systems(Startup, setup_menu)
            .add_systems(
                Update,
                (
                    handle_mode_selection,
                    handle_terrain_selection,
                    handle_back_button,
                    handle_start_game,
                    start_sandbox_immediately,
                    refresh_button_visuals,
                    rebuild_menu_on_phase_change,
                )
                    .run_if(menu_active),
            );
    }
}

/// Tracks previous menu phase to detect changes.
#[derive(Resource, Default)]
struct PreviousMenuPhase(Option<MenuPhase>);

/// Which phase of the menu flow we're in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MenuPhase {
    /// Choosing between Sandbox and Procedural modes.
    #[default]
    ModeSelection,
    /// Choosing terrain preset (Procedural mode only).
    TerrainSelection,
}

/// Track whether the main menu is active and current selections.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct MenuState {
    pub active: bool,
    pub phase: MenuPhase,
    pub selected_mode: Option<GameMode>,
    pub selected_terrain: TerrainPreset,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            active: true,
            phase: MenuPhase::ModeSelection,
            selected_mode: None,
            selected_terrain: TerrainPreset::Balanced,
        }
    }
}

/// Available terrain presets to start the simulation with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainPreset {
    Balanced,
    Coastal,
    Highlands,
}

impl TerrainPreset {
    fn label(&self) -> &'static str {
        match self {
            TerrainPreset::Balanced => "Balanced Plains",
            TerrainPreset::Coastal => "Coastal Delta",
            TerrainPreset::Highlands => "Highlands Ridge",
        }
    }

    fn subtitle(&self) -> &'static str {
        match self {
            TerrainPreset::Balanced => "Flat build area with gentle variation.",
            TerrainPreset::Coastal => "Rivers, inlets, and elevated plateaus.",
            TerrainPreset::Highlands => "Steeper slopes and wind corridors.",
        }
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct ModeButton(GameMode);

#[derive(Component)]
struct TerrainButton(TerrainPreset);

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct BackButton;

const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.94);
const PANEL: Color = Color::srgba(0.02, 0.03, 0.02, 0.97);
const BORDER: Color = Color::srgb(0.0, 0.75, 0.4);
const PRIMARY_TEXT: Color = Color::srgb(0.7, 1.0, 0.8);
const ACCENT_TEXT: Color = Color::srgb(1.0, 0.62, 0.2);
const MUTED_TEXT: Color = Color::srgb(0.5, 0.75, 0.6);
const BUTTON_IDLE: Color = Color::srgba(0.03, 0.05, 0.03, 0.98);
const BUTTON_HOVER: Color = Color::srgba(0.06, 0.09, 0.06, 0.98);
const BUTTON_SELECTED: Color = Color::srgba(0.08, 0.15, 0.1, 0.98);

fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu_state: Res<MenuState>,
    mut sim: ResMut<SimulationConfig>,
    mut tod: ResMut<TimeOfDay>,
) {
    // Pause world progression while in the menu.
    sim.paused = true;
    tod.paused = true;

    let font: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");

    spawn_menu_ui(&mut commands, &font, &menu_state);
}

/// Spawns the menu UI based on current phase.
fn spawn_menu_ui(commands: &mut Commands, font: &Handle<Font>, menu_state: &MenuState) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BACKDROP),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(720.0),
                    padding: UiRect::all(Val::Px(20.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    row_gap: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor(BORDER),
            ))
            .with_children(|panel| {
                match menu_state.phase {
                    MenuPhase::ModeSelection => {
                        spawn_mode_selection_content(panel, font);
                    }
                    MenuPhase::TerrainSelection => {
                        spawn_terrain_selection_content(panel, font, menu_state);
                    }
                }
            });
        });
}

/// Content for mode selection phase.
fn spawn_mode_selection_content(panel: &mut ChildBuilder, font: &Handle<Font>) {
    panel.spawn((
        Text::new("URBAN SPRAWL // NEW CITY"),
        TextFont {
            font: font.clone(),
            font_size: 22.0,
            ..default()
        },
        TextColor(ACCENT_TEXT),
    ));

    panel.spawn((
        Text::new("Choose how to start your city."),
        TextFont {
            font: font.clone(),
            font_size: 16.0,
            ..default()
        },
        TextColor(MUTED_TEXT),
    ));

    panel
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },))
        .with_children(|list| {
            // Sandbox mode button
            spawn_mode_button(
                list,
                GameMode::Sandbox,
                "SANDBOX MODE",
                "Start with a blank canvas. Build roads and zones from scratch.",
                "Full control | Manual roads | Build everything",
                font,
            );

            // Procedural mode button
            spawn_mode_button(
                list,
                GameMode::Procedural,
                "PROCEDURAL MODE",
                "Generate a road network to build upon. Classic city sim experience.",
                "Quick start | Generated roads | Focus on zoning",
                font,
            );
        });
}

/// Spawn a mode selection button.
fn spawn_mode_button(
    parent: &mut ChildBuilder,
    mode: GameMode,
    label: &str,
    description: &str,
    tags: &str,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.5)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            BorderColor(BORDER),
            ModeButton(mode),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(PRIMARY_TEXT),
            ));

            button.spawn((
                Text::new(description),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(MUTED_TEXT),
            ));

            button.spawn((
                Text::new(tags),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 0.6)),
            ));
        });
}

/// Content for terrain selection phase (Procedural mode only).
fn spawn_terrain_selection_content(
    panel: &mut ChildBuilder,
    font: &Handle<Font>,
    menu_state: &MenuState,
) {
    panel.spawn((
        Text::new("URBAN SPRAWL // TERRAIN SETUP"),
        TextFont {
            font: font.clone(),
            font_size: 22.0,
            ..default()
        },
        TextColor(ACCENT_TEXT),
    ));

    panel.spawn((
        Text::new("Calibrate your terrain before spinning up the city simulation."),
        TextFont {
            font: font.clone(),
            font_size: 16.0,
            ..default()
        },
        TextColor(MUTED_TEXT),
    ));

    panel
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },))
        .with_children(|list| {
            for preset in [
                TerrainPreset::Balanced,
                TerrainPreset::Coastal,
                TerrainPreset::Highlands,
            ] {
                spawn_terrain_button(list, preset, font, menu_state);
            }
        });

    panel
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        },))
        .with_children(|actions| {
            // Back button
            actions
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    BorderColor(BORDER),
                    BackButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("< BACK"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(MUTED_TEXT),
                    ));
                });

            // Start button
            actions
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.16, 0.12, 0.95)),
                    BorderColor(ACCENT_TEXT),
                    StartButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("START SIMULATION"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(PRIMARY_TEXT),
                    ));
                });
        });
}

fn spawn_terrain_button(
    parent: &mut ChildBuilder,
    preset: TerrainPreset,
    font: &Handle<Font>,
    menu_state: &MenuState,
) {
    let is_selected = menu_state.selected_terrain == preset;
    let background = if is_selected {
        BUTTON_SELECTED
    } else {
        BUTTON_IDLE
    };

    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.5)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(background),
            BorderColor(BORDER),
            TerrainButton(preset),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(preset.label()),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(PRIMARY_TEXT),
            ));

            button.spawn((
                Text::new(preset.subtitle()),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(MUTED_TEXT),
            ));

            let tags = match preset {
                TerrainPreset::Balanced => "Predictable rivers | Low erosion | Grid-friendly",
                TerrainPreset::Coastal => "Tidal flats | Mixed elevation | Scenic skylines",
                TerrainPreset::Highlands => "Ridge lines | Wind corridors | Dramatic vistas",
            };

            button.spawn((
                Text::new(tags),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 0.6)),
            ));
        });
}

/// Handle mode button clicks.
fn handle_mode_selection(
    mut menu_state: ResMut<MenuState>,
    interactions: Query<(&Interaction, &ModeButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, ModeButton(mode)) in &interactions {
        if *interaction == Interaction::Pressed {
            menu_state.selected_mode = Some(*mode);

            match mode {
                GameMode::Sandbox => {
                    // Sandbox mode: skip terrain selection, start immediately
                    // The rebuild system will trigger start
                }
                GameMode::Procedural => {
                    // Procedural mode: go to terrain selection
                    menu_state.phase = MenuPhase::TerrainSelection;
                }
                GameMode::None => {}
            }
        }
    }
}

fn handle_terrain_selection(
    mut menu_state: ResMut<MenuState>,
    interactions: Query<(&Interaction, &TerrainButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, TerrainButton(preset)) in &interactions {
        if *interaction == Interaction::Pressed {
            menu_state.selected_terrain = *preset;
        }
    }
}

/// Handle back button clicks.
fn handle_back_button(
    mut menu_state: ResMut<MenuState>,
    interactions: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            menu_state.phase = MenuPhase::ModeSelection;
            menu_state.selected_mode = None;
        }
    }
}

fn handle_start_game(
    mut commands: Commands,
    mut menu_state: ResMut<MenuState>,
    interactions: Query<&Interaction, (With<StartButton>, Changed<Interaction>)>,
    menu_roots: Query<Entity, With<MenuRoot>>,
    mut sim: ResMut<SimulationConfig>,
    mut tod: ResMut<TimeOfDay>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_game_mode: ResMut<NextState<GameMode>>,
) {
    if !menu_state.active {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            let mode = menu_state.selected_mode.unwrap_or(GameMode::Procedural);

            sim.paused = false;
            tod.paused = false;
            menu_state.active = false;

            // Set game state and mode
            next_game_state.set(GameState::Playing);
            next_game_mode.set(mode);

            for entity in &menu_roots {
                commands.entity(entity).despawn_recursive();
            }

            info!(
                "Starting simulation: mode={:?}, terrain={:?}",
                mode, menu_state.selected_terrain
            );
        }
    }
}

/// Start game immediately when Sandbox mode is selected.
fn start_sandbox_immediately(
    mut commands: Commands,
    mut menu_state: ResMut<MenuState>,
    menu_roots: Query<Entity, With<MenuRoot>>,
    mut sim: ResMut<SimulationConfig>,
    mut tod: ResMut<TimeOfDay>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_game_mode: ResMut<NextState<GameMode>>,
) {
    if !menu_state.active {
        return;
    }

    if menu_state.selected_mode == Some(GameMode::Sandbox) {
        sim.paused = false;
        tod.paused = false;
        menu_state.active = false;

        next_game_state.set(GameState::Playing);
        next_game_mode.set(GameMode::Sandbox);

        for entity in &menu_roots {
            commands.entity(entity).despawn_recursive();
        }

        info!("Starting Sandbox mode - blank canvas");
    }
}

fn refresh_button_visuals(
    menu_state: Res<MenuState>,
    mut terrain_buttons: Query<
        (
            &Interaction,
            &TerrainButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (With<Button>, Without<ModeButton>),
    >,
    mut mode_buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (With<Button>, With<ModeButton>, Without<TerrainButton>),
    >,
) {
    // Handle terrain button visuals
    for (interaction, TerrainButton(preset), mut background, mut border) in &mut terrain_buttons {
        let is_selected = menu_state.selected_terrain == *preset;

        background.0 = match *interaction {
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

        border.0 = if is_selected { ACCENT_TEXT } else { BORDER };
    }

    // Handle mode button hover states
    for (interaction, mut background, mut border) in &mut mode_buttons {
        background.0 = match *interaction {
            Interaction::Pressed => BUTTON_SELECTED,
            Interaction::Hovered => BUTTON_HOVER,
            Interaction::None => BUTTON_IDLE,
        };

        border.0 = match *interaction {
            Interaction::Hovered | Interaction::Pressed => ACCENT_TEXT,
            Interaction::None => BORDER,
        };
    }
}

/// Rebuild menu UI when phase changes.
fn rebuild_menu_on_phase_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu_state: Res<MenuState>,
    mut prev_phase: ResMut<PreviousMenuPhase>,
    menu_roots: Query<Entity, With<MenuRoot>>,
) {
    let current = Some(menu_state.phase);

    if prev_phase.0 != current {
        // Phase changed - rebuild the menu
        for entity in &menu_roots {
            commands.entity(entity).despawn_recursive();
        }

        let font: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");
        spawn_menu_ui(&mut commands, &font, &menu_state);

        prev_phase.0 = current;
    }
}

fn menu_active(menu_state: Option<Res<MenuState>>) -> bool {
    menu_state.map_or(false, |state| state.active)
}

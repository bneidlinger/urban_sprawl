//! Main menu and terrain selection overlay.

use bevy::prelude::*;

use crate::render::day_night::TimeOfDay;
use crate::simulation::SimulationConfig;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_systems(Startup, setup_menu)
            .add_systems(
                Update,
                (
                    handle_terrain_selection,
                    handle_start_game,
                    refresh_button_visuals,
                )
                    .run_if(menu_active),
            );
    }
}

/// Track whether the main menu is active and which terrain is selected.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct MenuState {
    pub active: bool,
    pub selected_terrain: TerrainPreset,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            active: true,
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
struct TerrainButton(TerrainPreset);

#[derive(Component)]
struct StartButton;

const BACKDROP: Color = Color::srgba(0.0, 0.02, 0.0, 0.9);
const PANEL: Color = Color::srgba(0.03, 0.05, 0.04, 0.95);
const BORDER: Color = Color::srgb(0.0, 0.65, 0.35);
const PRIMARY_TEXT: Color = Color::srgb(0.7, 1.0, 0.75);
const ACCENT_TEXT: Color = Color::srgb(1.0, 0.65, 0.3);
const MUTED_TEXT: Color = Color::srgb(0.65, 0.75, 0.68);
const BUTTON_IDLE: Color = Color::srgba(0.06, 0.08, 0.07, 0.95);
const BUTTON_HOVER: Color = Color::srgba(0.08, 0.12, 0.1, 0.95);
const BUTTON_SELECTED: Color = Color::srgba(0.1, 0.16, 0.12, 0.95);

fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut menu_state: ResMut<MenuState>,
    mut sim: ResMut<SimulationConfig>,
    mut tod: ResMut<TimeOfDay>,
) {
    // Pause world progression while in the menu.
    sim.paused = true;
    tod.paused = true;

    let font: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");

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
                panel.spawn((
                    Text::new("URBAN SPRAWL // START CONSOLE"),
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
                            spawn_terrain_button(list, preset, &font, &menu_state);
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
                        actions.spawn((
                            Text::new(
                                "Tip: switch presets anytime to preview different skyline flows.",
                            ),
                            TextFont {
                                font: font.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(MUTED_TEXT),
                        ));

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

fn handle_terrain_selection(
    mut menu_state: ResMut<MenuState>,
    mut interactions: Query<(&Interaction, &TerrainButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, TerrainButton(preset)) in &mut interactions {
        if *interaction == Interaction::Pressed {
            menu_state.selected_terrain = *preset;
        }
    }
}

fn handle_start_game(
    mut commands: Commands,
    mut menu_state: ResMut<MenuState>,
    mut interactions: Query<&Interaction, (With<StartButton>, Changed<Interaction>)>,
    menu_roots: Query<Entity, With<MenuRoot>>,
    mut sim: ResMut<SimulationConfig>,
    mut tod: ResMut<TimeOfDay>,
) {
    if menu_state.active {
        for interaction in &mut interactions {
            if *interaction == Interaction::Pressed {
                sim.paused = false;
                tod.paused = false;
                menu_state.active = false;

                for entity in &menu_roots {
                    commands.entity(entity).despawn_recursive();
                }

                info!(
                    "Starting simulation with {:?} preset",
                    menu_state.selected_terrain
                );
            }
        }
    }
}

fn refresh_button_visuals(
    menu_state: Res<MenuState>,
    mut buttons: Query<
        (
            &Interaction,
            &TerrainButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    for (interaction, TerrainButton(preset), mut background, mut border) in &mut buttons {
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
}

fn menu_active(menu_state: Option<Res<MenuState>>) -> bool {
    menu_state.map_or(false, |state| state.active)
}

//! Top stats bar showing city statistics and RCI demand.

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::simulation::demand::RCIDemand;
use crate::simulation::economy::CityBudget;
use crate::simulation::population::Population;

pub struct StatsBarPlugin;

impl Plugin for StatsBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_stats_bar)
            .add_systems(
                Update,
                update_stats_bar.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
struct StatsBarRoot;

#[derive(Component)]
struct PopulationText;

#[derive(Component)]
struct FundsText;

#[derive(Component)]
struct DemandMeter(DemandType);

#[derive(Clone, Copy)]
enum DemandType {
    Residential,
    Commercial,
    Industrial,
}

// Colors
const BAR_BG: Color = Color::srgba(0.03, 0.05, 0.04, 0.9);
const TEXT_COLOR: Color = Color::srgb(0.8, 0.95, 0.85);
const MUTED_TEXT: Color = Color::srgb(0.6, 0.7, 0.65);

fn setup_stats_bar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(70.0), // Offset from toolbox
                right: Val::Px(220.0), // Offset from HUD
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(BAR_BG),
            StatsBarRoot,
        ))
        .with_children(|bar| {
            // Population section
            bar.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },))
            .with_children(|section| {
                section.spawn((
                    Text::new("POP:"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(MUTED_TEXT),
                ));
                section.spawn((
                    Text::new("0"),
                    TextFont {
                        font: font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    PopulationText,
                ));
            });

            // Funds section
            bar.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },))
            .with_children(|section| {
                section.spawn((
                    Text::new("$"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.3, 0.9, 0.4)),
                ));
                section.spawn((
                    Text::new("50,000"),
                    TextFont {
                        font: font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    FundsText,
                ));
            });

            // RCI Demand meters
            bar.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },))
            .with_children(|section| {
                spawn_demand_meter(section, &font, "R", DemandType::Residential, Color::srgb(0.2, 0.8, 0.3));
                spawn_demand_meter(section, &font, "C", DemandType::Commercial, Color::srgb(0.3, 0.5, 0.9));
                spawn_demand_meter(section, &font, "I", DemandType::Industrial, Color::srgb(0.9, 0.7, 0.2));
            });
        });
}

fn spawn_demand_meter(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    label: &str,
    demand_type: DemandType,
    color: Color,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|meter| {
            // Label
            meter.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(color),
            ));

            // Meter background
            meter
                .spawn((Node {
                    width: Val::Px(40.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                ))
                .with_children(|bg| {
                    // Meter fill
                    bg.spawn((
                        Node {
                            width: Val::Percent(50.0), // Will be updated
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(color),
                        DemandMeter(demand_type),
                    ));
                });
        });
}

fn update_stats_bar(
    population: Res<Population>,
    budget: Res<CityBudget>,
    demand: Res<RCIDemand>,
    mut pop_text: Query<&mut Text, (With<PopulationText>, Without<FundsText>)>,
    mut funds_text: Query<&mut Text, (With<FundsText>, Without<PopulationText>)>,
    mut meters: Query<(&DemandMeter, &mut Node, &mut BackgroundColor)>,
) {
    // Update population
    for mut text in &mut pop_text {
        **text = format_number(population.total as i64);
    }

    // Update funds
    for mut text in &mut funds_text {
        **text = format_number(budget.funds);
    }

    // Update demand meters
    for (meter, mut node, mut bg) in &mut meters {
        let demand_value = match meter.0 {
            DemandType::Residential => demand.residential,
            DemandType::Commercial => demand.commercial,
            DemandType::Industrial => demand.industrial,
        };

        // Convert -1..1 to 0..100%
        let percent = ((demand_value + 1.0) / 2.0 * 100.0).clamp(0.0, 100.0);
        node.width = Val::Percent(percent);

        // Color intensity based on demand
        let base_color = match meter.0 {
            DemandType::Residential => Color::srgb(0.2, 0.8, 0.3),
            DemandType::Commercial => Color::srgb(0.3, 0.5, 0.9),
            DemandType::Industrial => Color::srgb(0.9, 0.7, 0.2),
        };

        // Dim if negative demand, bright if positive
        let alpha = if demand_value > 0.0 {
            0.8 + demand_value * 0.2
        } else {
            0.3 + (1.0 + demand_value) * 0.5
        };
        bg.0 = base_color.with_alpha(alpha);
    }
}

fn format_number(n: i64) -> String {
    if n.abs() >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n.abs() >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

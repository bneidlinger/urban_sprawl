//! Demolish tool - remove buildings, zones, and roads.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::{ActiveTool, ToolState};
use crate::game_state::GameState;
use crate::render::building_spawner::Building;
use crate::simulation::zones::GrownBuilding;
use crate::tools::road_draw::RoadMeshDirty;
use crate::tools::zone_paint::{ZoneCell, ZoneGrid};

pub struct DemolishPlugin;

impl Plugin for DemolishPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemolishConfig>()
            .add_systems(
                Update,
                (handle_demolish_input, update_demolish_preview, apply_demolish)
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(is_demolish_active),
            );
    }
}

/// Run condition: check if demolish tool is active.
fn is_demolish_active(tool: Res<State<ActiveTool>>) -> bool {
    matches!(tool.get(), ActiveTool::Demolish)
}

/// Configuration for demolish tool.
#[derive(Resource)]
pub struct DemolishConfig {
    /// Radius for demolish area.
    pub demolish_radius: f32,
    /// Cost per building demolished.
    pub cost_per_building: i64,
    /// Cost per zone cell cleared.
    pub cost_per_zone: i64,
}

impl Default for DemolishConfig {
    fn default() -> Self {
        Self {
            demolish_radius: 5.0,
            cost_per_building: 100,
            cost_per_zone: 10,
        }
    }
}

/// Marker for demolish preview.
#[derive(Component)]
struct DemolishPreview;

/// Marker for highlighted demolish targets.
#[derive(Component)]
struct DemolishHighlight;

fn handle_demolish_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut tool_state: ResMut<ToolState>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    // Get cursor position in world space
    let Some(cursor_pos) = window.cursor_position() else {
        tool_state.drag_end = None;
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        tool_state.drag_end = None;
        return;
    };

    // Update current position for preview
    tool_state.drag_end = Some(world_pos);

    // Track dragging for area demolish
    if mouse.just_pressed(MouseButton::Left) {
        tool_state.is_dragging = true;
        tool_state.drag_start = Some(world_pos);
    }

    if mouse.just_released(MouseButton::Left) {
        tool_state.is_dragging = false;
    }
}

fn update_demolish_preview(
    mut commands: Commands,
    tool_state: Res<ToolState>,
    config: Res<DemolishConfig>,
    mut preview_q: Query<(Entity, &mut Transform, &mut Sprite), With<DemolishPreview>>,
) {
    let Some(pos) = tool_state.drag_end else {
        // Remove preview when not hovering
        for (entity, _, _) in &preview_q {
            commands.entity(entity).despawn();
        }
        return;
    };

    let size = if tool_state.is_dragging {
        // Drag rectangle
        if let Some(start) = tool_state.drag_start {
            let min = start.min(pos);
            let max = start.max(pos);
            max - min
        } else {
            Vec2::splat(config.demolish_radius * 2.0)
        }
    } else {
        Vec2::splat(config.demolish_radius * 2.0)
    };

    let center = if tool_state.is_dragging {
        if let Some(start) = tool_state.drag_start {
            (start + pos) / 2.0
        } else {
            pos
        }
    } else {
        pos
    };

    if let Some((_, mut transform, mut sprite)) = preview_q.iter_mut().next() {
        transform.translation = Vec3::new(center.x, 0.4, center.y);
        sprite.custom_size = Some(size);
    } else {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.9, 0.2, 0.2, 0.4),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(Vec3::new(center.x, 0.4, center.y))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            DemolishPreview,
        ));
    }
}

fn apply_demolish(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    tool_state: Res<ToolState>,
    config: Res<DemolishConfig>,
    mut zone_grid: ResMut<ZoneGrid>,
    mut budget: ResMut<crate::simulation::economy::CityBudget>,
    buildings: Query<(Entity, &GlobalTransform), With<Building>>,
    grown_buildings: Query<(Entity, &GrownBuilding)>,
    zone_cells: Query<(Entity, &ZoneCell, &GlobalTransform)>,
    mut dirty_events: EventWriter<RoadMeshDirty>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }

    let Some(end_pos) = tool_state.drag_end else {
        return;
    };

    // Get demolish area
    let (min, max) = if let Some(start) = tool_state.drag_start {
        (start.min(end_pos), start.max(end_pos))
    } else {
        let half = config.demolish_radius;
        (end_pos - Vec2::splat(half), end_pos + Vec2::splat(half))
    };

    let mut buildings_demolished = 0;
    let mut zones_cleared = 0;
    let mut total_cost = 0i64;

    // Demolish buildings in area
    for (entity, transform) in &buildings {
        let pos = transform.translation();
        let pos_2d = Vec2::new(pos.x, pos.z);

        if pos_2d.x >= min.x && pos_2d.x <= max.x && pos_2d.y >= min.y && pos_2d.y <= max.y {
            commands.entity(entity).despawn_recursive();
            buildings_demolished += 1;
            total_cost += config.cost_per_building;
        }
    }

    // Also check grown buildings and update their zone cells
    for (entity, grown) in &grown_buildings {
        if let Ok((_, transform)) = buildings.get(entity) {
            let pos = transform.translation();
            let pos_2d = Vec2::new(pos.x, pos.z);

            if pos_2d.x >= min.x && pos_2d.x <= max.x && pos_2d.y >= min.y && pos_2d.y <= max.y {
                // The building entity was already despawned above if it has Building component
                // Just need to clear the zone cell reference
                if let Ok((zone_entity, _, _)) = zone_cells.get(grown.zone_cell) {
                    commands.entity(zone_entity).try_insert(ZoneCellCleared);
                }
            }
        }
    }

    // Demolish zone cells in area
    for (entity, cell, transform) in &zone_cells {
        let pos = transform.translation();
        let pos_2d = Vec2::new(pos.x, pos.z);

        if pos_2d.x >= min.x && pos_2d.x <= max.x && pos_2d.y >= min.y && pos_2d.y <= max.y {
            // Remove from grid
            zone_grid.cells.remove(&cell.grid_pos);

            // Despawn building if any
            if let Some(building_entity) = cell.building {
                commands.entity(building_entity).despawn_recursive();
                buildings_demolished += 1;
                total_cost += config.cost_per_building;
            }

            // Despawn zone cell
            commands.entity(entity).despawn_recursive();
            zones_cleared += 1;
            total_cost += config.cost_per_zone;
        }
    }

    // Deduct cost from budget
    budget.funds -= total_cost;

    if buildings_demolished > 0 || zones_cleared > 0 {
        info!(
            "Demolished {} buildings, cleared {} zones. Cost: ${}",
            buildings_demolished, zones_cleared, total_cost
        );

        // Trigger mesh updates if needed
        dirty_events.send(RoadMeshDirty);
    }
}

/// Marker for zone cells that need their building reference cleared.
#[derive(Component)]
struct ZoneCellCleared;

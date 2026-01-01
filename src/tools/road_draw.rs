//! Road drawing tool - click to place road nodes and edges.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use smallvec::SmallVec;

use super::ActiveTool;
use crate::game_state::GameState;
use crate::procgen::roads::{RoadGraph, RoadNodeType, RoadType};

pub struct RoadDrawPlugin;

impl Plugin for RoadDrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadDrawState>()
            .init_resource::<RoadDrawConfig>()
            .add_event::<RoadMeshDirty>()
            .add_systems(
                Update,
                (
                    handle_road_draw_input,
                    update_road_preview,
                    cleanup_on_tool_change,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Event to trigger road mesh regeneration.
#[derive(Event)]
pub struct RoadMeshDirty;

/// Configuration for road drawing.
#[derive(Resource)]
pub struct RoadDrawConfig {
    /// Distance to snap to existing nodes.
    pub snap_distance: f32,
    /// Current road type being drawn.
    pub road_type: RoadType,
    /// Preview node size.
    pub preview_size: f32,
}

impl Default for RoadDrawConfig {
    fn default() -> Self {
        Self {
            snap_distance: 15.0,
            road_type: RoadType::Minor,
            preview_size: 3.0,
        }
    }
}

/// State for road drawing tool.
#[derive(Resource, Default)]
pub struct RoadDrawState {
    /// Previous node placed (for connecting).
    pub last_node: Option<petgraph::graph::NodeIndex>,
    /// Current hover position.
    pub hover_pos: Option<Vec2>,
    /// Whether we're snapping to an existing node.
    pub snapping_to: Option<petgraph::graph::NodeIndex>,
}

/// Marker for road preview entities.
#[derive(Component)]
struct RoadPreview;

/// Marker for node preview entity.
#[derive(Component)]
struct NodePreview;

/// Marker for edge preview entity.
#[derive(Component)]
struct EdgePreview;

/// Run condition: check if road draw tool is active.
fn is_road_draw_active(tool: &State<ActiveTool>) -> bool {
    matches!(tool.get(), ActiveTool::RoadDraw)
}

fn handle_road_draw_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    tool: Res<State<ActiveTool>>,
    mut road_graph: ResMut<RoadGraph>,
    mut state: ResMut<RoadDrawState>,
    config: Res<RoadDrawConfig>,
    mut dirty_events: EventWriter<RoadMeshDirty>,
) {
    if !is_road_draw_active(&tool) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    // Get cursor position in world space
    let Some(cursor_pos) = window.cursor_position() else {
        state.hover_pos = None;
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        state.hover_pos = None;
        return;
    };

    state.hover_pos = Some(world_pos);

    // Check if we're snapping to an existing node
    state.snapping_to = road_graph.find_nearest(world_pos, config.snap_distance);

    // Place node on left click
    if mouse.just_pressed(MouseButton::Left) {
        let new_node = road_graph.snap_or_create(
            world_pos,
            config.snap_distance,
            if state.last_node.is_some() {
                RoadNodeType::Intersection
            } else {
                RoadNodeType::Endpoint
            },
        );

        // Connect to previous node if exists
        if let Some(prev_node) = state.last_node {
            if new_node != prev_node {
                // Get positions for the edge
                let prev_pos = road_graph
                    .node_by_index(prev_node)
                    .map(|n| n.position)
                    .unwrap_or(world_pos);
                let new_pos = road_graph
                    .node_by_index(new_node)
                    .map(|n| n.position)
                    .unwrap_or(world_pos);

                // Create edge with just start/end points
                let points: SmallVec<[Vec2; 8]> = SmallVec::from_slice(&[prev_pos, new_pos]);
                road_graph.add_edge(prev_node, new_node, points, config.road_type);

                info!(
                    "Road edge placed: {:?} -> {:?} ({:?})",
                    prev_pos, new_pos, config.road_type
                );

                // Trigger mesh regeneration
                dirty_events.send(RoadMeshDirty);
            }
        } else {
            info!("Road node placed at {:?}", world_pos);
        }

        state.last_node = Some(new_node);
    }

    // Cancel/finish with right click or Escape
    if mouse.just_pressed(MouseButton::Right) || keyboard.just_pressed(KeyCode::Escape) {
        if state.last_node.is_some() {
            info!("Road drawing ended");
            state.last_node = None;
        }
    }
}

fn update_road_preview(
    mut commands: Commands,
    state: Res<RoadDrawState>,
    tool: Res<State<ActiveTool>>,
    config: Res<RoadDrawConfig>,
    road_graph: Res<RoadGraph>,
    mut node_preview: Query<(Entity, &mut Transform), (With<NodePreview>, Without<EdgePreview>)>,
    mut edge_preview: Query<(Entity, &mut Transform, &mut Sprite), With<EdgePreview>>,
) {
    if !is_road_draw_active(&tool) {
        // Clean up previews when tool not active
        for (entity, _, _) in &edge_preview {
            commands.entity(entity).despawn();
        }
        for (entity, _) in &node_preview {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Some(hover_pos) = state.hover_pos else {
        // Clean up previews when not hovering
        for (entity, _, _) in &edge_preview {
            commands.entity(entity).despawn();
        }
        for (entity, _) in &node_preview {
            commands.entity(entity).despawn();
        }
        return;
    };

    // Get actual position (snapped or raw)
    let actual_pos = if let Some(snap_node) = state.snapping_to {
        road_graph
            .node_by_index(snap_node)
            .map(|n| n.position)
            .unwrap_or(hover_pos)
    } else {
        hover_pos
    };

    // Node preview color (green for new, yellow for snap)
    let node_color = if state.snapping_to.is_some() {
        Color::srgba(1.0, 0.9, 0.2, 0.8) // Yellow for snapping
    } else {
        Color::srgba(0.3, 0.9, 0.4, 0.8) // Green for new
    };

    // Update or spawn node preview
    if let Some((_, mut transform)) = node_preview.iter_mut().next() {
        transform.translation = Vec3::new(actual_pos.x, 0.5, actual_pos.y);
    } else {
        commands.spawn((
            Sprite {
                color: node_color,
                custom_size: Some(Vec2::splat(config.preview_size * 2.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(actual_pos.x, 0.5, actual_pos.y))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            NodePreview,
            RoadPreview,
        ));
    }

    // Edge preview if we have a previous node
    if let Some(prev_node) = state.last_node {
        let Some(prev_pos) = road_graph.node_by_index(prev_node).map(|n| n.position) else {
            return;
        };

        let center = (prev_pos + actual_pos) / 2.0;
        let length = prev_pos.distance(actual_pos);
        let angle = (actual_pos - prev_pos).to_angle();

        let road_width = match config.road_type {
            RoadType::Highway => 12.0,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => 3.0,
        };

        if let Some((_, mut transform, mut sprite)) = edge_preview.iter_mut().next() {
            transform.translation = Vec3::new(center.x, 0.3, center.y);
            transform.rotation = Quat::from_rotation_y(-angle);
            sprite.custom_size = Some(Vec2::new(length, road_width));
        } else {
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.4, 0.4, 0.5, 0.6),
                    custom_size: Some(Vec2::new(length, road_width)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(center.x, 0.3, center.y))
                    .with_rotation(
                        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                            * Quat::from_rotation_z(-angle),
                    ),
                EdgePreview,
                RoadPreview,
            ));
        }
    } else {
        // No previous node, clean up edge preview
        for (entity, _, _) in &edge_preview {
            commands.entity(entity).despawn();
        }
    }
}

fn cleanup_on_tool_change(
    mut commands: Commands,
    tool: Res<State<ActiveTool>>,
    mut state: ResMut<RoadDrawState>,
    previews: Query<Entity, With<RoadPreview>>,
) {
    if !tool.is_changed() {
        return;
    }

    if !is_road_draw_active(&tool) {
        // Reset state when switching away from road tool
        state.last_node = None;
        state.hover_pos = None;
        state.snapping_to = None;

        // Despawn all previews
        for entity in &previews {
            commands.entity(entity).despawn();
        }
    }
}

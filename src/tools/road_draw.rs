//! Road drawing tool - click to place road nodes and edges.
//!
//! Supports both straight and curved (bezier) road drawing.
//! Includes undo/redo functionality (Ctrl+Z / Ctrl+Y).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use petgraph::graph::NodeIndex;
use smallvec::SmallVec;

use super::ActiveTool;
use crate::game_state::GameState;
use crate::procgen::roads::{RoadEdge, RoadGraph, RoadNodeType, RoadType};

pub struct RoadDrawPlugin;

impl Plugin for RoadDrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadDrawState>()
            .init_resource::<RoadDrawConfig>()
            .init_resource::<RoadHistory>()
            .add_event::<RoadMeshDirty>()
            .add_systems(
                Update,
                (
                    handle_draw_mode_toggle,
                    handle_undo_redo,
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

/// Draw mode for roads.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum RoadDrawMode {
    /// Straight line between points.
    #[default]
    Straight,
    /// Quadratic bezier curve (drag to define curve).
    Curved,
}

/// Configuration for road drawing.
#[derive(Resource)]
pub struct RoadDrawConfig {
    /// Distance to snap to existing nodes.
    pub snap_distance: f32,
    /// Current road type being drawn.
    pub road_type: RoadType,
    /// Preview node size.
    pub preview_size: f32,
    /// Current draw mode (straight or curved).
    pub draw_mode: RoadDrawMode,
    /// Number of segments in a bezier curve.
    pub curve_segments: usize,
}

impl Default for RoadDrawConfig {
    fn default() -> Self {
        Self {
            snap_distance: 15.0,
            road_type: RoadType::Minor,
            preview_size: 3.0,
            draw_mode: RoadDrawMode::Straight,
            curve_segments: 8,
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
    /// For curved mode: start position when dragging.
    pub curve_start: Option<Vec2>,
    /// For curved mode: is the user currently dragging to define curve?
    pub is_dragging: bool,
    /// For curved mode: the control point offset perpendicular to the line.
    pub curve_offset: f32,
}

/// An undoable road action.
#[derive(Clone, Debug)]
pub enum RoadAction {
    /// A node was added (stores position, type, and the assigned index).
    AddNode {
        position: Vec2,
        node_type: RoadNodeType,
        node_index: NodeIndex,
    },
    /// An edge was added between two nodes.
    AddEdge {
        from: NodeIndex,
        to: NodeIndex,
        edge: RoadEdge,
    },
    /// A compound action: node + edge added together.
    AddNodeAndEdge {
        node_position: Vec2,
        node_type: RoadNodeType,
        node_index: NodeIndex,
        from: NodeIndex,
        edge: RoadEdge,
    },
}

/// History of road actions for undo/redo.
#[derive(Resource)]
pub struct RoadHistory {
    /// Stack of actions that can be undone.
    undo_stack: Vec<RoadAction>,
    /// Stack of actions that can be redone.
    redo_stack: Vec<RoadAction>,
    /// Maximum history size.
    max_size: usize,
}

impl RoadHistory {
    /// Push a new action to the history. Clears redo stack.
    pub fn push(&mut self, action: RoadAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();

        // Limit history size
        if self.max_size > 0 && self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Push to undo stack without clearing redo (for redo operations).
    fn push_undo_only(&mut self, action: RoadAction) {
        self.undo_stack.push(action);

        // Limit history size
        if self.max_size > 0 && self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Pop an action for undo. Returns None if nothing to undo.
    pub fn pop_undo(&mut self) -> Option<RoadAction> {
        self.undo_stack.pop()
    }

    /// Push an action to redo stack.
    pub fn push_redo(&mut self, action: RoadAction) {
        self.redo_stack.push(action);
    }

    /// Pop an action for redo. Returns None if nothing to redo.
    pub fn pop_redo(&mut self) -> Option<RoadAction> {
        self.redo_stack.pop()
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for RoadHistory {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 100, // Keep last 100 actions
        }
    }
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

/// Toggle draw mode with 'B' key.
fn handle_draw_mode_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    tool: Res<State<ActiveTool>>,
    mut config: ResMut<RoadDrawConfig>,
) {
    if !is_road_draw_active(&tool) {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        config.draw_mode = match config.draw_mode {
            RoadDrawMode::Straight => {
                info!("Road draw mode: Curved (Bezier)");
                RoadDrawMode::Curved
            }
            RoadDrawMode::Curved => {
                info!("Road draw mode: Straight");
                RoadDrawMode::Straight
            }
        };
    }
}

/// Handle undo (Ctrl+Z) and redo (Ctrl+Y or Ctrl+Shift+Z).
fn handle_undo_redo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut road_graph: ResMut<RoadGraph>,
    mut history: ResMut<RoadHistory>,
    mut state: ResMut<RoadDrawState>,
    mut dirty_events: EventWriter<RoadMeshDirty>,
) {
    let ctrl_pressed = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift_pressed = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Undo: Ctrl+Z
    if ctrl_pressed && keyboard.just_pressed(KeyCode::KeyZ) && !shift_pressed {
        if let Some(action) = history.pop_undo() {
            // Apply reverse of the action
            match &action {
                RoadAction::AddNode { node_index, .. } => {
                    // Remove the node (this also removes connected edges)
                    road_graph.remove_node(*node_index);
                    // Clear last_node if it was this node
                    if state.last_node == Some(*node_index) {
                        state.last_node = None;
                    }
                    info!("Undo: removed node");
                }
                RoadAction::AddEdge { from, to, .. } => {
                    // Find and remove the edge between these nodes
                    if let Some(edge_idx) = road_graph.find_edge(*from, *to) {
                        road_graph.remove_edge(edge_idx);
                        info!("Undo: removed edge");
                    }
                }
                RoadAction::AddNodeAndEdge { node_index, from, .. } => {
                    // First remove the edge, then the node
                    if let Some(edge_idx) = road_graph.find_edge(*from, *node_index) {
                        road_graph.remove_edge(edge_idx);
                    }
                    road_graph.remove_node(*node_index);
                    // Clear last_node if it was this node
                    if state.last_node == Some(*node_index) {
                        state.last_node = Some(*from); // Go back to previous node
                    }
                    info!("Undo: removed node and edge");
                }
            }

            // Save for redo
            history.push_redo(action);
            dirty_events.send(RoadMeshDirty);
        }
    }

    // Redo: Ctrl+Y or Ctrl+Shift+Z
    if ctrl_pressed && (keyboard.just_pressed(KeyCode::KeyY) || (shift_pressed && keyboard.just_pressed(KeyCode::KeyZ))) {
        if let Some(action) = history.pop_redo() {
            // Re-apply the action and create updated action with new indices
            let new_action = match &action {
                RoadAction::AddNode { position, node_type, .. } => {
                    let new_idx = road_graph.add_node(*position, *node_type);
                    state.last_node = Some(new_idx);
                    info!("Redo: added node");
                    RoadAction::AddNode {
                        position: *position,
                        node_type: *node_type,
                        node_index: new_idx,
                    }
                }
                RoadAction::AddEdge { from, to, edge } => {
                    road_graph.add_edge_data(*from, *to, edge.clone());
                    info!("Redo: added edge");
                    action.clone()
                }
                RoadAction::AddNodeAndEdge { node_position, node_type, from, edge, .. } => {
                    let new_idx = road_graph.add_node(*node_position, *node_type);
                    road_graph.add_edge_data(*from, new_idx, edge.clone());
                    state.last_node = Some(new_idx);
                    info!("Redo: added node and edge");
                    RoadAction::AddNodeAndEdge {
                        node_position: *node_position,
                        node_type: *node_type,
                        node_index: new_idx,
                        from: *from,
                        edge: edge.clone(),
                    }
                }
            };

            // Push back to undo stack without clearing redo
            history.push_undo_only(new_action);
            dirty_events.send(RoadMeshDirty);
        }
    }
}

/// Generate points along a quadratic bezier curve.
fn generate_bezier_points(start: Vec2, control: Vec2, end: Vec2, segments: usize) -> SmallVec<[Vec2; 8]> {
    let mut points = SmallVec::new();
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        // Quadratic bezier: B(t) = (1-t)²P0 + 2(1-t)tP1 + t²P2
        let one_minus_t = 1.0 - t;
        let point = one_minus_t * one_minus_t * start
            + 2.0 * one_minus_t * t * control
            + t * t * end;
        points.push(point);
    }
    points
}

/// Calculate the control point for a curved road based on cursor offset.
fn calculate_control_point(start: Vec2, end: Vec2, offset: f32) -> Vec2 {
    let midpoint = (start + end) / 2.0;
    let direction = (end - start).normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    midpoint + perpendicular * offset
}

fn handle_road_draw_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    tool: Res<State<ActiveTool>>,
    mut road_graph: ResMut<RoadGraph>,
    mut state: ResMut<RoadDrawState>,
    mut history: ResMut<RoadHistory>,
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

    // Get snapped position
    let actual_pos = if let Some(snap_node) = state.snapping_to {
        road_graph
            .node_by_index(snap_node)
            .map(|n| n.position)
            .unwrap_or(world_pos)
    } else {
        world_pos
    };

    match config.draw_mode {
        RoadDrawMode::Straight => {
            // Straight mode: click to place nodes, auto-connect to previous
            if mouse.just_pressed(MouseButton::Left) {
                // Check if we're snapping to existing or creating new
                let snapping = state.snapping_to.is_some();
                let node_type = if state.last_node.is_some() {
                    RoadNodeType::Intersection
                } else {
                    RoadNodeType::Endpoint
                };

                let new_node = road_graph.snap_or_create(world_pos, config.snap_distance, node_type);

                // Connect to previous node if exists
                if let Some(prev_node) = state.last_node {
                    if new_node != prev_node {
                        let prev_pos = road_graph
                            .node_by_index(prev_node)
                            .map(|n| n.position)
                            .unwrap_or(world_pos);
                        let new_pos = road_graph
                            .node_by_index(new_node)
                            .map(|n| n.position)
                            .unwrap_or(world_pos);

                        let points: SmallVec<[Vec2; 8]> = SmallVec::from_slice(&[prev_pos, new_pos]);
                        let edge = RoadEdge::new(points.clone(), config.road_type);
                        road_graph.add_edge(prev_node, new_node, points, config.road_type);

                        // Record action for undo
                        if snapping {
                            // Just added an edge to existing node
                            history.push(RoadAction::AddEdge {
                                from: prev_node,
                                to: new_node,
                                edge,
                            });
                        } else {
                            // Added new node + edge
                            history.push(RoadAction::AddNodeAndEdge {
                                node_position: new_pos,
                                node_type,
                                node_index: new_node,
                                from: prev_node,
                                edge,
                            });
                        }

                        info!(
                            "Road edge placed: {:?} -> {:?} ({:?})",
                            prev_pos, new_pos, config.road_type
                        );

                        dirty_events.send(RoadMeshDirty);
                    }
                } else {
                    // First node placed
                    if !snapping {
                        history.push(RoadAction::AddNode {
                            position: actual_pos,
                            node_type,
                            node_index: new_node,
                        });
                    }
                    info!("Road node placed at {:?}", world_pos);
                }

                state.last_node = Some(new_node);
            }
        }
        RoadDrawMode::Curved => {
            // Curved mode: click to start, drag to define curve, release to place
            if mouse.just_pressed(MouseButton::Left) {
                if let Some(prev_node) = state.last_node {
                    // We have a previous node - start dragging to define curve
                    let prev_pos = road_graph
                        .node_by_index(prev_node)
                        .map(|n| n.position)
                        .unwrap_or(world_pos);
                    state.curve_start = Some(prev_pos);
                    state.is_dragging = true;
                    state.curve_offset = 0.0;
                } else {
                    // No previous node - just place the first node
                    let snapping = state.snapping_to.is_some();
                    let new_node = road_graph.snap_or_create(
                        world_pos,
                        config.snap_distance,
                        RoadNodeType::Endpoint,
                    );

                    // Record first node for undo (if not snapping to existing)
                    if !snapping {
                        history.push(RoadAction::AddNode {
                            position: actual_pos,
                            node_type: RoadNodeType::Endpoint,
                            node_index: new_node,
                        });
                    }

                    state.last_node = Some(new_node);
                    info!("Road node placed at {:?}", world_pos);
                }
            }

            // While dragging, calculate curve offset
            if state.is_dragging {
                if let Some(start) = state.curve_start {
                    // Calculate perpendicular distance from the line to cursor
                    let line_dir = (actual_pos - start).normalize_or_zero();
                    let to_cursor = world_pos - start;
                    let perpendicular = Vec2::new(-line_dir.y, line_dir.x);
                    state.curve_offset = to_cursor.dot(perpendicular);
                }
            }

            // On release, create the curved edge
            if mouse.just_released(MouseButton::Left) && state.is_dragging {
                if let Some(prev_node) = state.last_node {
                    let snapping = state.snapping_to.is_some();
                    let new_node = road_graph.snap_or_create(
                        actual_pos,
                        config.snap_distance,
                        RoadNodeType::Intersection,
                    );

                    if new_node != prev_node {
                        let prev_pos = road_graph
                            .node_by_index(prev_node)
                            .map(|n| n.position)
                            .unwrap_or(world_pos);
                        let new_pos = road_graph
                            .node_by_index(new_node)
                            .map(|n| n.position)
                            .unwrap_or(world_pos);

                        // Generate bezier curve points
                        let control = calculate_control_point(prev_pos, new_pos, state.curve_offset);
                        let points = generate_bezier_points(prev_pos, control, new_pos, config.curve_segments);
                        let edge = RoadEdge::new(points.clone(), config.road_type);

                        road_graph.add_edge(prev_node, new_node, points, config.road_type);

                        // Record action for undo
                        if snapping {
                            history.push(RoadAction::AddEdge {
                                from: prev_node,
                                to: new_node,
                                edge,
                            });
                        } else {
                            history.push(RoadAction::AddNodeAndEdge {
                                node_position: new_pos,
                                node_type: RoadNodeType::Intersection,
                                node_index: new_node,
                                from: prev_node,
                                edge,
                            });
                        }

                        info!(
                            "Curved road edge placed: {:?} -> {:?} (offset: {:.1})",
                            prev_pos, new_pos, state.curve_offset
                        );

                        dirty_events.send(RoadMeshDirty);
                    }

                    state.last_node = Some(new_node);
                }

                state.is_dragging = false;
                state.curve_start = None;
                state.curve_offset = 0.0;
            }
        }
    }

    // Cancel/finish with right click or Escape
    if mouse.just_pressed(MouseButton::Right) || keyboard.just_pressed(KeyCode::Escape) {
        if state.last_node.is_some() || state.is_dragging {
            info!("Road drawing ended");
            state.last_node = None;
            state.is_dragging = false;
            state.curve_start = None;
            state.curve_offset = 0.0;
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

        // Different color for curved mode (purple) vs straight (grey)
        let edge_color = match config.draw_mode {
            RoadDrawMode::Straight => Color::srgba(0.4, 0.4, 0.5, 0.6),
            RoadDrawMode::Curved => Color::srgba(0.6, 0.3, 0.7, 0.6),
        };

        if let Some((_, mut transform, mut sprite)) = edge_preview.iter_mut().next() {
            transform.translation = Vec3::new(center.x, 0.3, center.y);
            transform.rotation = Quat::from_rotation_y(-angle);
            sprite.custom_size = Some(Vec2::new(length, road_width));
            sprite.color = edge_color;
        } else {
            commands.spawn((
                Sprite {
                    color: edge_color,
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
        // Reset all state when switching away from road tool
        state.last_node = None;
        state.hover_pos = None;
        state.snapping_to = None;
        state.curve_start = None;
        state.is_dragging = false;
        state.curve_offset = 0.0;

        // Despawn all previews
        for entity in &previews {
            commands.entity(entity).despawn();
        }
    }
}

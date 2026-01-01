//! Zone painting tool - click-drag to paint rectangular zone areas.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::{ActiveTool, ToolState, ZoneType};
use crate::game_state::GameState;

pub struct ZonePaintPlugin;

impl Plugin for ZonePaintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneGrid>()
            .init_resource::<ZonePaintConfig>()
            .add_systems(
                Update,
                (
                    handle_zone_tool_input,
                    update_zone_preview,
                    apply_zone_on_release,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(is_zone_tool_active),
            )
            .add_systems(Update, spawn_zone_visuals.run_if(in_state(GameState::Playing)));
    }
}

/// Run condition: check if a zone tool is active.
fn is_zone_tool_active(tool: Res<State<ActiveTool>>) -> bool {
    matches!(tool.get(), ActiveTool::ZonePaint(_))
}

/// Configuration for zone painting.
#[derive(Resource)]
pub struct ZonePaintConfig {
    /// Size of each zone cell in world units.
    pub cell_size: f32,
    /// Preview opacity.
    pub preview_alpha: f32,
}

impl Default for ZonePaintConfig {
    fn default() -> Self {
        Self {
            cell_size: 10.0,
            preview_alpha: 0.4,
        }
    }
}

/// A single zoned cell on the grid.
#[derive(Component, Clone)]
pub struct ZoneCell {
    /// Grid position (not world position).
    pub grid_pos: IVec2,
    /// Zone type assigned to this cell.
    pub zone_type: ZoneType,
    /// Development level: 0 = empty, 1-3 = building density.
    pub development_level: u8,
    /// Building entity if developed.
    pub building: Option<Entity>,
}

/// Grid of all zone cells.
#[derive(Resource, Default)]
pub struct ZoneGrid {
    /// All zone cells by grid position.
    pub cells: std::collections::HashMap<IVec2, Entity>,
}

impl ZoneGrid {
    /// Get cell entity at grid position.
    pub fn get(&self, pos: IVec2) -> Option<Entity> {
        self.cells.get(&pos).copied()
    }

    /// Insert or update cell at grid position.
    pub fn insert(&mut self, pos: IVec2, entity: Entity) {
        self.cells.insert(pos, entity);
    }

    /// Remove cell at grid position.
    pub fn remove(&mut self, pos: IVec2) -> Option<Entity> {
        self.cells.remove(&pos)
    }

    /// Check if position is zoned.
    pub fn is_zoned(&self, pos: IVec2) -> bool {
        self.cells.contains_key(&pos)
    }
}

/// Marker for zone preview visualization.
#[derive(Component)]
struct ZonePreview;

/// Marker for zone cell visualization mesh.
#[derive(Component)]
pub struct ZoneCellVisual;

/// Handle mouse input for zone painting.
fn handle_zone_tool_input(
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
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Start drag on left mouse down
    if mouse.just_pressed(MouseButton::Left) {
        tool_state.is_dragging = true;
        tool_state.drag_start = Some(world_pos);
        tool_state.drag_end = Some(world_pos);
    }

    // Update drag end while holding
    if mouse.pressed(MouseButton::Left) && tool_state.is_dragging {
        tool_state.drag_end = Some(world_pos);
    }

    // End drag on release (handled in apply_zone_on_release)
    if mouse.just_released(MouseButton::Left) {
        tool_state.is_dragging = false;
    }
}

/// Update preview rectangle while dragging.
fn update_zone_preview(
    mut commands: Commands,
    tool_state: Res<ToolState>,
    active_tool: Res<State<ActiveTool>>,
    config: Res<ZonePaintConfig>,
    mut preview_q: Query<(Entity, &mut Transform, &mut Sprite), With<ZonePreview>>,
) {
    let zone_type = match active_tool.get() {
        ActiveTool::ZonePaint(zt) => *zt,
        _ => return,
    };

    if !tool_state.is_dragging {
        // Remove preview when not dragging
        for (entity, _, _) in &preview_q {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Some(rect) = tool_state.drag_rect() else {
        return;
    };

    // Snap to grid
    let min_grid = world_to_grid(rect.min, config.cell_size);
    let max_grid = world_to_grid(rect.max, config.cell_size);
    let snapped_min = grid_to_world(min_grid, config.cell_size);
    let snapped_max = grid_to_world(max_grid + IVec2::ONE, config.cell_size);

    let center = (snapped_min + snapped_max) / 2.0;
    let size = snapped_max - snapped_min;

    let color = zone_color(zone_type).with_alpha(config.preview_alpha);

    if let Some((_, mut transform, mut sprite)) = preview_q.iter_mut().next() {
        // Update existing preview
        transform.translation = Vec3::new(center.x, 0.2, center.y);
        sprite.custom_size = Some(size);
        sprite.color = color;
    } else {
        // Spawn new preview
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(Vec3::new(center.x, 0.2, center.y))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ZonePreview,
        ));
    }
}

/// Apply zones when mouse is released.
fn apply_zone_on_release(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    mut tool_state: ResMut<ToolState>,
    active_tool: Res<State<ActiveTool>>,
    config: Res<ZonePaintConfig>,
    mut zone_grid: ResMut<ZoneGrid>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }

    let zone_type = match active_tool.get() {
        ActiveTool::ZonePaint(zt) => *zt,
        _ => return,
    };

    let Some(rect) = tool_state.drag_rect() else {
        tool_state.drag_start = None;
        tool_state.drag_end = None;
        return;
    };

    // Snap to grid
    let min_grid = world_to_grid(rect.min, config.cell_size);
    let max_grid = world_to_grid(rect.max, config.cell_size);

    let mut cells_created = 0;

    // Create zone cells for each grid position
    for gx in min_grid.x..=max_grid.x {
        for gy in min_grid.y..=max_grid.y {
            let grid_pos = IVec2::new(gx, gy);

            // Skip if already zoned
            if zone_grid.is_zoned(grid_pos) {
                continue;
            }

            let world_pos = grid_to_world(grid_pos, config.cell_size);
            let center = world_pos + Vec2::splat(config.cell_size / 2.0);

            // Create zone cell entity
            let entity = commands
                .spawn((
                    ZoneCell {
                        grid_pos,
                        zone_type,
                        development_level: 0,
                        building: None,
                    },
                    Transform::from_translation(Vec3::new(center.x, 0.05, center.y)),
                    Visibility::default(),
                ))
                .id();

            zone_grid.insert(grid_pos, entity);
            cells_created += 1;
        }
    }

    if cells_created > 0 {
        info!(
            "Zoned {} cells as {:?} from {:?} to {:?}",
            cells_created, zone_type, min_grid, max_grid
        );
    }

    // Clear drag state
    tool_state.drag_start = None;
    tool_state.drag_end = None;
}

/// Spawn visual meshes for zone cells that don't have them yet.
fn spawn_zone_visuals(
    mut commands: Commands,
    config: Res<ZonePaintConfig>,
    zone_cells: Query<(Entity, &ZoneCell), Without<ZoneCellVisual>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, cell) in &zone_cells {
        let color = zone_color(cell.zone_type).with_alpha(0.6);

        // Create a flat plane mesh for the zone cell
        let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(config.cell_size * 0.45)));
        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        commands.entity(entity).insert((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            ZoneCellVisual,
        ));
    }
}

/// Convert world position to grid position.
fn world_to_grid(world: Vec2, cell_size: f32) -> IVec2 {
    IVec2::new(
        (world.x / cell_size).floor() as i32,
        (world.y / cell_size).floor() as i32,
    )
}

/// Convert grid position to world position (bottom-left corner of cell).
fn grid_to_world(grid: IVec2, cell_size: f32) -> Vec2 {
    Vec2::new(grid.x as f32 * cell_size, grid.y as f32 * cell_size)
}

/// Get the color for a zone type.
pub fn zone_color(zone_type: ZoneType) -> Color {
    match zone_type {
        ZoneType::Residential => Color::srgb(0.2, 0.8, 0.3), // Green
        ZoneType::Commercial => Color::srgb(0.3, 0.5, 0.9),  // Blue
        ZoneType::Industrial => Color::srgb(0.9, 0.7, 0.2),  // Yellow/Orange
        ZoneType::Civic => Color::srgb(0.7, 0.3, 0.8),       // Purple
        ZoneType::Green => Color::srgb(0.1, 0.6, 0.2),       // Dark Green
    }
}

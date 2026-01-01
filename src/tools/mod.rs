//! Player tools for interacting with the city.
//!
//! Tools allow the player to modify the city: zoning land, drawing roads,
//! demolishing buildings, and placing services.

use bevy::prelude::*;

pub mod demolish;
pub mod road_draw;
pub mod services;
pub mod zone_paint;

pub use crate::procgen::lot_engine::ZoneType;
pub use services::ServiceType;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ActiveTool>()
            .init_resource::<ToolState>()
            .add_plugins(zone_paint::ZonePaintPlugin)
            .add_plugins(road_draw::RoadDrawPlugin)
            .add_plugins(demolish::DemolishPlugin)
            .add_plugins(services::ServicesPlugin);
    }
}

/// Currently active tool.
#[derive(States, Default, Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum ActiveTool {
    /// No tool selected - default camera controls.
    #[default]
    None,
    /// Zone painting tool - click-drag to paint zones.
    ZonePaint(ZoneType),
    /// Road drawing tool - click to place nodes.
    RoadDraw,
    /// Demolish tool - click to remove buildings/roads.
    Demolish,
    /// Service placement tool.
    PlaceService(ServiceType),
    /// Query tool - click to inspect objects.
    Query,
}

/// Shared state for tool interactions.
#[derive(Resource, Default)]
pub struct ToolState {
    /// Whether the user is currently dragging.
    pub is_dragging: bool,
    /// World position where drag started.
    pub drag_start: Option<Vec2>,
    /// Current drag end position.
    pub drag_end: Option<Vec2>,
    /// Brush size for painting tools.
    pub brush_size: f32,
}

impl ToolState {
    /// Get the drag rectangle if a drag is in progress.
    pub fn drag_rect(&self) -> Option<Rect> {
        match (self.drag_start, self.drag_end) {
            (Some(start), Some(end)) => {
                let min = start.min(end);
                let max = start.max(end);
                Some(Rect::from_corners(min, max))
            }
            _ => None,
        }
    }
}

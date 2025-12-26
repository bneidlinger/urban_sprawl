//! Debug rendering for roads and tensor fields using Bevy gizmos.

use bevy::prelude::*;

use crate::procgen::road_generator::RoadsGenerated;
use crate::procgen::roads::{RoadGraph, RoadType};
use crate::procgen::tensor::TensorField;
use crate::ui::DebugConfig;

pub struct DebugRenderPlugin;

impl Plugin for DebugRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (render_roads, render_tensor_field));
    }
}

/// Render roads as gizmo lines.
fn render_roads(
    road_graph: Res<RoadGraph>,
    generated: Res<RoadsGenerated>,
    config: Res<DebugConfig>,
    mut gizmos: Gizmos,
) {
    if !generated.0 || !config.show_road_graph {
        return;
    }

    // Draw edges
    for edge in road_graph.edges() {
        let color = match edge.road_type {
            RoadType::Highway => Color::srgb(1.0, 0.8, 0.0),
            RoadType::Major => Color::srgb(1.0, 1.0, 1.0),
            RoadType::Minor => Color::srgb(0.7, 0.7, 0.7),
            RoadType::Alley => Color::srgb(0.4, 0.4, 0.4),
        };

        // Draw road segments
        for window in edge.points.windows(2) {
            let start = Vec3::new(window[0].x, 0.5, window[0].y);
            let end = Vec3::new(window[1].x, 0.5, window[1].y);
            gizmos.line(start, end, color);
        }
    }

    // Draw nodes as small spheres (using short lines as cross markers)
    for (_idx, node) in road_graph.nodes() {
        let pos = Vec3::new(node.position.x, 0.5, node.position.y);
        let size = 1.5;

        let node_color = match node.node_type {
            crate::procgen::roads::RoadNodeType::Intersection => Color::srgb(0.0, 1.0, 0.0),
            crate::procgen::roads::RoadNodeType::Endpoint => Color::srgb(1.0, 0.0, 0.0),
            crate::procgen::roads::RoadNodeType::DeadEnd => Color::srgb(1.0, 0.5, 0.0),
        };

        // Draw cross marker
        gizmos.line(pos + Vec3::X * size, pos - Vec3::X * size, node_color);
        gizmos.line(pos + Vec3::Z * size, pos - Vec3::Z * size, node_color);
    }
}

/// Render tensor field as directional lines.
fn render_tensor_field(
    tensor_field: Res<TensorField>,
    config: Res<DebugConfig>,
    mut gizmos: Gizmos,
) {
    if !config.show_tensor_field {
        return;
    }

    let grid_spacing = 20.0;
    let half_size = 250.0;
    let line_length = 8.0;

    let mut y = -half_size;
    while y <= half_size {
        let mut x = -half_size;
        while x <= half_size {
            let pos = Vec2::new(x, y);
            let tensor = tensor_field.sample(pos);

            let major = tensor.major() * line_length;
            let minor = tensor.minor() * line_length * 0.5;

            let world_pos = Vec3::new(x, 1.0, y);

            // Major direction (white)
            gizmos.line(
                world_pos - Vec3::new(major.x, 0.0, major.y),
                world_pos + Vec3::new(major.x, 0.0, major.y),
                Color::srgb(1.0, 1.0, 1.0),
            );

            // Minor direction (cyan, shorter)
            gizmos.line(
                world_pos - Vec3::new(minor.x, 0.0, minor.y),
                world_pos + Vec3::new(minor.x, 0.0, minor.y),
                Color::srgb(0.0, 1.0, 1.0),
            );

            x += grid_spacing;
        }
        y += grid_spacing;
    }
}

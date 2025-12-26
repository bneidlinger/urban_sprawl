//! City block extraction from road graph.
//!
//! Uses a grid-based approach to find buildable areas between roads.

use bevy::prelude::*;

use super::parcels::Lot;
use super::roads::RoadGraph;
use super::road_generator::RoadsGenerated;

pub struct BlockExtractorPlugin;

impl Plugin for BlockExtractorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityBlocks>()
            .init_resource::<CityLots>()
            .init_resource::<BlockConfig>()
            .add_systems(Update, extract_blocks.run_if(should_extract_blocks));
    }
}

fn should_extract_blocks(
    generated: Res<RoadsGenerated>,
    blocks: Res<CityBlocks>,
) -> bool {
    generated.0 && !blocks.extracted
}

/// Configuration for block extraction.
#[derive(Resource)]
pub struct BlockConfig {
    pub grid_cell_size: f32,
    pub road_clearance: f32,  // Min distance from road center
    pub city_half_size: f32,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            grid_cell_size: 12.0,   // Size of each potential lot
            road_clearance: 8.0,    // Stay this far from road centerline
            city_half_size: 250.0,
        }
    }
}

/// Resource containing extracted city blocks (unused in grid approach).
#[derive(Resource, Default)]
pub struct CityBlocks {
    pub blocks: Vec<Vec<Vec2>>,
    pub extracted: bool,
}

/// Resource containing buildable lots.
#[derive(Resource, Default)]
pub struct CityLots {
    pub lots: Vec<Lot>,
}

/// Extract buildable lots using grid-based approach.
fn extract_blocks(
    road_graph: Res<RoadGraph>,
    config: Res<BlockConfig>,
    mut blocks: ResMut<CityBlocks>,
    mut lots: ResMut<CityLots>,
) {
    info!("Extracting buildable lots...");

    // Collect all road segment points for distance checking
    let road_points: Vec<Vec2> = collect_road_points(&road_graph, 2.0);

    let mut valid_lots = Vec::new();
    let half = config.city_half_size;
    let cell = config.grid_cell_size;
    let clearance = config.road_clearance;

    // Grid over the city
    let mut y = -half;
    while y < half {
        let mut x = -half;
        while x < half {
            let cell_center = Vec2::new(x + cell / 2.0, y + cell / 2.0);

            // Check if this cell is far enough from all roads
            let min_dist = min_distance_to_roads(&cell_center, &road_points);

            if min_dist > clearance {
                // Create a lot at this position
                let half_cell = cell / 2.0 - 1.0; // Slight gap between lots
                let vertices = vec![
                    Vec2::new(x + 1.0, y + 1.0),
                    Vec2::new(x + cell - 1.0, y + 1.0),
                    Vec2::new(x + cell - 1.0, y + cell - 1.0),
                    Vec2::new(x + 1.0, y + cell - 1.0),
                ];

                valid_lots.push(Lot {
                    vertices,
                    area: (cell - 2.0) * (cell - 2.0),
                    frontage: None,
                });
            }

            x += cell;
        }
        y += cell;
    }

    info!("Found {} buildable lots", valid_lots.len());

    blocks.extracted = true;
    lots.lots = valid_lots;
}

/// Collect points along all road edges for distance checking.
fn collect_road_points(graph: &RoadGraph, spacing: f32) -> Vec<Vec2> {
    let mut points = Vec::new();

    for edge in graph.edges() {
        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let dir = end - start;
            let len = dir.length();

            if len < 0.001 {
                continue;
            }

            let step = dir.normalize() * spacing;
            let steps = (len / spacing).ceil() as usize;

            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                points.push(start + dir * t);
            }
        }
    }

    // Also add node positions
    for (_idx, node) in graph.nodes() {
        points.push(node.position);
    }

    points
}

/// Find minimum distance from a point to any road point.
fn min_distance_to_roads(point: &Vec2, road_points: &[Vec2]) -> f32 {
    road_points
        .iter()
        .map(|rp| point.distance(*rp))
        .fold(f32::MAX, f32::min)
}

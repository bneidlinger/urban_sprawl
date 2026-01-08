//! River generation for the city.
//!
//! Creates a meandering river that flows through the city using Perlin noise.
//! Only generates in Procedural mode - Sandbox mode starts with a blank terrain.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::game_state::GameMode;

pub struct RiverPlugin;

impl Plugin for RiverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RiverConfig>()
            .init_resource::<River>()
            .init_resource::<RiverGenerated>()
            // Generate river when entering Procedural mode (not at Startup)
            .add_systems(OnEnter(GameMode::Procedural), generate_river)
            // Mark as generated (empty) when entering Sandbox mode
            .add_systems(OnEnter(GameMode::Sandbox), skip_river_generation);
    }
}

/// Mark river as generated (but empty) for Sandbox mode.
fn skip_river_generation(mut generated: ResMut<RiverGenerated>) {
    generated.0 = true;
    info!("Sandbox mode - no river generated");
}

/// Configuration for river generation.
#[derive(Resource, Clone)]
pub struct RiverConfig {
    /// Enable/disable river generation.
    pub enabled: bool,
    /// Random seed for river path.
    pub seed: u32,
    /// Base width of the river in world units.
    pub river_width: f32,
    /// Random variation in width (0.0-1.0).
    pub width_variation: f32,
    /// Amplitude of river meandering.
    pub meander_amplitude: f32,
    /// Frequency of river bends.
    pub meander_frequency: f32,
    /// Height of water surface.
    pub water_level: f32,
    /// Width of sloped banks.
    pub bank_slope_width: f32,
    /// City size (should match terrain).
    pub city_size: f32,
    /// Number of points along river centerline.
    pub resolution: usize,
}

impl Default for RiverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            seed: 12345,
            river_width: 30.0,
            width_variation: 0.3,
            meander_amplitude: 80.0,
            meander_frequency: 0.015,
            water_level: -2.0,
            bank_slope_width: 15.0,
            city_size: 500.0,
            resolution: 100,
        }
    }
}

/// A point along the river centerline.
#[derive(Clone, Debug)]
pub struct RiverPoint {
    /// Position in world coordinates.
    pub position: Vec2,
    /// Width of river at this point.
    pub width: f32,
    /// Tangent direction (normalized).
    pub direction: Vec2,
}

/// The river resource containing the generated river data.
#[derive(Resource, Default)]
pub struct River {
    /// Points along the river centerline.
    pub centerline: Vec<RiverPoint>,
    /// Left bank polyline (looking downstream).
    pub left_bank: Vec<Vec2>,
    /// Right bank polyline.
    pub right_bank: Vec<Vec2>,
    /// Water surface height.
    pub water_level: f32,
    /// Bank slope width for terrain carving.
    pub bank_slope_width: f32,
    /// Axis-aligned bounding box for quick rejection.
    pub bounds: Rect,
}

impl River {
    /// Check if a 2D point is inside the river (between banks).
    pub fn contains_point(&self, point: Vec2) -> bool {
        // Quick bounding box rejection
        if !self.bounds.contains(point) {
            return false;
        }

        // Check signed distance
        self.signed_distance(point) < 0.0
    }

    /// Get signed distance from point to river edge.
    /// Negative = inside river, Positive = outside river.
    pub fn signed_distance(&self, point: Vec2) -> f32 {
        if self.centerline.is_empty() {
            return f32::MAX;
        }

        let mut min_dist = f32::MAX;

        // Find closest point on centerline and compute distance to edge
        for river_point in &self.centerline {
            let dist_to_center = point.distance(river_point.position);
            let dist_to_edge = dist_to_center - river_point.width * 0.5;
            min_dist = min_dist.min(dist_to_edge);
        }

        min_dist
    }

    /// Check if a line segment intersects the river.
    /// Returns the first intersection point if any.
    pub fn intersects_segment(&self, start: Vec2, end: Vec2) -> Option<Vec2> {
        if self.left_bank.len() < 2 {
            return None;
        }

        // Check against both banks
        for bank in [&self.left_bank, &self.right_bank] {
            for window in bank.windows(2) {
                if let Some(intersection) = segment_intersection(start, end, window[0], window[1]) {
                    return Some(intersection);
                }
            }
        }

        None
    }

    /// Get all intersection points of a polyline with the river.
    /// Returns (segment_index, intersection_point) pairs.
    pub fn intersect_polyline(&self, points: &[Vec2]) -> Vec<(usize, Vec2)> {
        let mut intersections = Vec::new();

        for (i, window) in points.windows(2).enumerate() {
            // Check both entry and exit points
            if let Some(pt) = self.intersects_segment(window[0], window[1]) {
                intersections.push((i, pt));
            }
        }

        intersections
    }

    /// Check if a road segment crosses the river completely.
    /// Returns (entry_point, exit_point) if it does.
    pub fn crosses_river(&self, start: Vec2, end: Vec2) -> Option<(Vec2, Vec2)> {
        let start_inside = self.contains_point(start);
        let end_inside = self.contains_point(end);

        // Must start outside, cross in, and exit
        if start_inside || end_inside {
            return None;
        }

        // Find intersections with both banks
        let mut intersections = Vec::new();

        for bank in [&self.left_bank, &self.right_bank] {
            for window in bank.windows(2) {
                if let Some(pt) = segment_intersection(start, end, window[0], window[1]) {
                    intersections.push(pt);
                }
            }
        }

        // Should have exactly 2 intersections for a complete crossing
        if intersections.len() >= 2 {
            // Sort by distance from start
            intersections.sort_by(|a, b| {
                start.distance(*a).partial_cmp(&start.distance(*b)).unwrap()
            });
            Some((intersections[0], intersections[1]))
        } else {
            None
        }
    }
}

/// Marker resource indicating river has been generated.
#[derive(Resource, Default)]
pub struct RiverGenerated(pub bool);

/// Generate the river at startup.
fn generate_river(
    config: Res<RiverConfig>,
    mut river: ResMut<River>,
    mut generated: ResMut<RiverGenerated>,
) {
    if !config.enabled {
        generated.0 = true;
        info!("River generation disabled");
        return;
    }

    let perlin = Perlin::new(config.seed);
    let width_perlin = Perlin::new(config.seed.wrapping_add(1000));

    let half_size = config.city_size * 0.5;
    let mut centerline: Vec<RiverPoint> = Vec::with_capacity(config.resolution);

    // River flows from bottom-left to top-right (roughly diagonal)
    // Start position with some randomness
    let start_offset = perlin.get([0.0, 0.0]) as f32 * 50.0;
    let start = Vec2::new(-half_size, -half_size * 0.7 + start_offset);
    let end = Vec2::new(half_size, half_size * 0.7 - start_offset);

    let base_direction = (end - start).normalize();

    for i in 0..config.resolution {
        let t = i as f32 / (config.resolution - 1) as f32;

        // Base position along the diagonal
        let base_pos = start.lerp(end, t);

        // Add meandering using Perlin noise
        let noise_val = perlin.get([t as f64 * 10.0, config.seed as f64 * 0.1]) as f32;
        let perpendicular = Vec2::new(-base_direction.y, base_direction.x);
        let meander_offset = perpendicular * noise_val * config.meander_amplitude;

        let position = base_pos + meander_offset;

        // Vary width along river
        let width_noise = width_perlin.get([t as f64 * 5.0, 0.0]) as f32;
        let width = config.river_width * (1.0 + width_noise * config.width_variation);

        // Calculate direction (tangent)
        let direction = if i == 0 {
            base_direction
        } else {
            (position - centerline[i - 1].position).normalize()
        };

        centerline.push(RiverPoint {
            position,
            width,
            direction,
        });
    }

    // Smooth directions by averaging with neighbors
    for i in 1..centerline.len() - 1 {
        let prev = centerline[i - 1].position;
        let next = centerline[i + 1].position;
        centerline[i].direction = (next - prev).normalize();
    }

    // Generate bank polylines
    let mut left_bank = Vec::with_capacity(config.resolution);
    let mut right_bank = Vec::with_capacity(config.resolution);

    for point in &centerline {
        let perpendicular = Vec2::new(-point.direction.y, point.direction.x);
        let half_width = point.width * 0.5;

        left_bank.push(point.position + perpendicular * half_width);
        right_bank.push(point.position - perpendicular * half_width);
    }

    // Calculate bounding box
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);

    for point in left_bank.iter().chain(right_bank.iter()) {
        min = min.min(*point);
        max = max.max(*point);
    }

    // Expand bounds slightly for bank slopes
    min -= Vec2::splat(config.bank_slope_width);
    max += Vec2::splat(config.bank_slope_width);

    river.centerline = centerline;
    river.left_bank = left_bank;
    river.right_bank = right_bank;
    river.water_level = config.water_level;
    river.bank_slope_width = config.bank_slope_width;
    river.bounds = Rect::from_corners(min, max);

    generated.0 = true;

    info!(
        "River generated with {} points, width ~{:.0}m, water level {:.1}m",
        river.centerline.len(),
        config.river_width,
        config.water_level
    );
}

/// Line segment intersection test.
/// Returns intersection point if segments intersect.
fn segment_intersection(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> Option<Vec2> {
    let d1 = a2 - a1;
    let d2 = b2 - b1;

    let cross = d1.x * d2.y - d1.y * d2.x;

    // Parallel lines
    if cross.abs() < 1e-10 {
        return None;
    }

    let d = b1 - a1;
    let t = (d.x * d2.y - d.y * d2.x) / cross;
    let u = (d.x * d1.y - d.y * d1.x) / cross;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(a1 + d1 * t)
    } else {
        None
    }
}

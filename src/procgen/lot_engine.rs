//! Lot planning and building growth engine.
//!
//! Consumes the buildable lots discovered between roads and annotates them
//! with zoning, density, and environmental context. The intent is to provide
//! a predictable-yet-randomized plan that later systems can use to spawn
#![allow(dead_code)]
//! buildings, parks, or civic spaces.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{rngs::StdRng, Rng, SeedableRng};

use super::block_extractor::CityLots;
use super::parcels::Lot;
use super::roads::RoadGraph;

pub struct LotEnginePlugin;

impl Plugin for LotEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LotEngineConfig>()
            .init_resource::<LotPlans>()
            .add_systems(Update, plan_open_space_lots.run_if(should_plan_lots));
    }
}

/// Settings for how lots should be evaluated.
#[derive(Resource)]
pub struct LotEngineConfig {
    /// Approximate city radius used to bias density near the center.
    pub city_radius: f32,
    /// How far road influence should reach when evaluating access/noise.
    pub max_road_influence: f32,
    /// Density score threshold for promoting lots to high density.
    pub high_density_cutoff: f32,
    /// Density score threshold for promoting lots to medium density.
    pub medium_density_cutoff: f32,
    /// Scale for procedural noise sampling (smaller = smoother fields).
    pub env_noise_scale: f64,
    /// Seed to keep growth planning deterministic between runs.
    pub seed: u64,
}

impl Default for LotEngineConfig {
    fn default() -> Self {
        Self {
            city_radius: 250.0,
            max_road_influence: 40.0,
            high_density_cutoff: 0.65,
            medium_density_cutoff: 0.35,
            env_noise_scale: 0.03,
            seed: 42,
        }
    }
}

/// Classification for density-aware planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DensityTier {
    Low,
    Medium,
    High,
}

/// Proposed zoning for a lot.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ZoneType {
    Residential,
    Commercial,
    Industrial,
    Civic,
    Green,
}

/// Environmental modifiers affecting desirability and growth cadence.
#[derive(Clone, Copy, Debug)]
pub struct EnvironmentalFactors {
    pub sunlight: f32,
    pub greenery: f32,
    pub noise: f32,
}

/// A lot annotated with planning data for downstream building generation.
#[derive(Clone, Debug)]
pub struct PlannedLot {
    pub lot: Lot,
    pub centroid: Vec2,
    pub density: DensityTier,
    pub zone: ZoneType,
    pub environment: EnvironmentalFactors,
    /// Likelihood of attempting construction during the next review window.
    pub build_probability: f32,
    /// Number of in-sim days before the lot reevaluates growth.
    pub next_review_in_days: u32,
}

/// Collection of planned lots generated from open spaces between roads.
#[derive(Resource, Default)]
pub struct LotPlans {
    pub planned: Vec<PlannedLot>,
    pub generated: bool,
}

fn should_plan_lots(lots: Res<CityLots>, plans: Res<LotPlans>) -> bool {
    !lots.lots.is_empty() && !plans.generated
}

fn plan_open_space_lots(
    road_graph: Res<RoadGraph>,
    lots: Res<CityLots>,
    config: Res<LotEngineConfig>,
    mut plans: ResMut<LotPlans>,
) {
    info!("Planning zoning and growth targets for open lots");

    let mut rng = StdRng::seed_from_u64(config.seed);
    // Perlin expects a u32 seed; fold the configured seed into the expected width for deterministic noise
    let perlin_seed = config.seed as u32;
    let perlin = Perlin::new(perlin_seed);
    let road_points = collect_road_points(&road_graph, 3.0);

    let mut annotated = Vec::new();

    for lot in &lots.lots {
        let centroid = polygon_centroid(&lot.vertices);
        let distance_from_center = centroid.length();
        let road_distance = min_distance_to_roads(&centroid, &road_points);

        let environment = evaluate_environment(
            centroid,
            road_distance,
            &perlin,
            config.env_noise_scale,
            config.max_road_influence,
        );

        let density_score = density_score(
            distance_from_center,
            road_distance,
            lot.area,
            environment,
            config.city_radius,
            config.max_road_influence,
            &mut rng,
        );
        let density = classify_density(
            density_score,
            config.high_density_cutoff,
            config.medium_density_cutoff,
        );
        let zone = choose_zone(
            density,
            environment,
            road_distance,
            config.max_road_influence,
        );
        let build_probability = compute_growth_probability(density, environment, &mut rng);
        let next_review_in_days = schedule_next_review(density, &mut rng);

        annotated.push(PlannedLot {
            lot: lot.clone(),
            centroid,
            density,
            zone,
            environment,
            build_probability,
            next_review_in_days,
        });
    }

    plans.planned = annotated;
    plans.generated = true;
}

fn evaluate_environment(
    centroid: Vec2,
    road_distance: f32,
    perlin: &Perlin,
    noise_scale: f64,
    max_road_influence: f32,
) -> EnvironmentalFactors {
    let sample = [
        centroid.x as f64 * noise_scale,
        centroid.y as f64 * noise_scale,
    ];
    let sunlight = normalize_noise(perlin.get(sample));
    let greenery = normalize_noise(perlin.get([sample[0] + 15.0, sample[1] + 42.0]));
    let noise = (1.0 - (road_distance / max_road_influence).clamp(0.0, 1.0)).clamp(0.0, 1.0);

    EnvironmentalFactors {
        sunlight,
        greenery,
        noise,
    }
}

fn density_score(
    distance_from_center: f32,
    road_distance: f32,
    lot_area: f32,
    environment: EnvironmentalFactors,
    city_radius: f32,
    max_road_influence: f32,
    rng: &mut StdRng,
) -> f32 {
    let center_bias = 1.0 - (distance_from_center / city_radius).clamp(0.0, 1.0);
    let road_bias = (max_road_influence - road_distance) / max_road_influence;
    let env_bias = 0.5 * environment.sunlight + 0.5 * environment.greenery;
    let area_bias = (200.0 / lot_area.max(50.0)).clamp(0.0, 1.0);
    let randomness: f32 = rng.gen_range(0.0..0.15);

    (center_bias * 0.45
        + road_bias.clamp(0.0, 1.0) * 0.3
        + env_bias * 0.15
        + area_bias * 0.1
        + randomness)
        .clamp(0.0, 1.2)
}

fn classify_density(score: f32, high_cutoff: f32, medium_cutoff: f32) -> DensityTier {
    if score >= high_cutoff {
        DensityTier::High
    } else if score >= medium_cutoff {
        DensityTier::Medium
    } else {
        DensityTier::Low
    }
}

fn choose_zone(
    density: DensityTier,
    environment: EnvironmentalFactors,
    road_distance: f32,
    max_road_influence: f32,
) -> ZoneType {
    let near_roads = (road_distance / max_road_influence).clamp(0.0, 1.0) < 0.45;

    if environment.greenery > 0.65 && density != DensityTier::High {
        ZoneType::Green
    } else if environment.noise > 0.75 {
        ZoneType::Industrial
    } else if density == DensityTier::High && near_roads {
        ZoneType::Commercial
    } else if density == DensityTier::Medium && near_roads {
        ZoneType::Civic
    } else {
        ZoneType::Residential
    }
}

fn compute_growth_probability(
    density: DensityTier,
    environment: EnvironmentalFactors,
    rng: &mut StdRng,
) -> f32 {
    let base = match density {
        DensityTier::High => 0.65,
        DensityTier::Medium => 0.5,
        DensityTier::Low => 0.35,
    };

    let environment_bonus =
        (environment.sunlight * 0.2) + (environment.greenery * 0.1) - (environment.noise * 0.15);
    let randomness: f32 = rng.gen_range(-0.05..0.1);

    (base + environment_bonus + randomness).clamp(0.05, 0.95)
}

fn schedule_next_review(density: DensityTier, rng: &mut StdRng) -> u32 {
    let (min_days, max_days) = match density {
        DensityTier::High => (15, 45),
        DensityTier::Medium => (30, 90),
        DensityTier::Low => (60, 150),
    };

    rng.gen_range(min_days..=max_days)
}

fn polygon_centroid(vertices: &[Vec2]) -> Vec2 {
    if vertices.is_empty() {
        return Vec2::ZERO;
    }

    let mut centroid = Vec2::ZERO;
    for v in vertices {
        centroid += *v;
    }
    centroid / vertices.len() as f32
}

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

            let _step = dir.normalize() * spacing;
            let steps = (len / spacing).ceil() as usize;

            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                points.push(start + dir * t);
            }
        }
    }

    for (_idx, node) in graph.nodes() {
        points.push(node.position);
    }

    points
}

fn min_distance_to_roads(point: &Vec2, road_points: &[Vec2]) -> f32 {
    road_points
        .iter()
        .map(|rp| point.distance(*rp))
        .fold(f32::MAX, f32::min)
}

fn normalize_noise(value: f64) -> f32 {
    (((value as f32) + 1.0) * 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_classification_respects_thresholds() {
        assert_eq!(classify_density(0.8, 0.7, 0.4), DensityTier::High);
        assert_eq!(classify_density(0.6, 0.7, 0.4), DensityTier::Medium);
        assert_eq!(classify_density(0.2, 0.7, 0.4), DensityTier::Low);
    }

    #[test]
    fn growth_probability_is_clamped() {
        let env = EnvironmentalFactors {
            sunlight: 1.0,
            greenery: 1.0,
            noise: 0.0,
        };
        let mut rng = StdRng::seed_from_u64(1);
        let probability = compute_growth_probability(DensityTier::High, env, &mut rng);
        assert!(probability >= 0.05 && probability <= 0.95);
    }
}

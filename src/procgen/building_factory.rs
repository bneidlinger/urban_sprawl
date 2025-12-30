//! Building factory that transforms planned lots into deterministic building blueprints.
//!
//! The factory consumes zoning and density information from [`LotPlans`]
//! and emits per-lot building or park plans. This keeps visual spawning
//! simple while ensuring growable zones produce appropriately scaled
//! footprints, floor counts, and façade styles.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::procgen::lot_engine::{DensityTier, LotPlans, PlannedLot, ZoneType};
use crate::procgen::lot_geometry::{polygon_bounds, shrink_polygon};

pub struct BuildingFactoryPlugin;

impl Plugin for BuildingFactoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingFactoryConfig>()
            .init_resource::<BuildingBlueprints>()
            .add_systems(Update, plan_blueprints.run_if(should_plan_blueprints));
    }
}

#[derive(Resource)]
pub struct BuildingFactoryConfig {
    /// How far buildings should be inset from lot edges.
    pub setback: f32,
    /// Minimum usable footprint after setbacks.
    pub min_footprint: f32,
    /// Area scale used to bias taller buildings on larger lots.
    pub large_lot_area: f32,
    /// Seed to keep planning deterministic between runs.
    pub seed: u64,
    /// Maximum percentage of the shrunken lot to use as a building footprint.
    pub coverage_variance: f32,
}

impl Default for BuildingFactoryConfig {
    fn default() -> Self {
        Self {
            setback: 2.0,
            min_footprint: 3.5,
            large_lot_area: 140.0,
            seed: 31415,
            coverage_variance: 0.15,
        }
    }
}

/// High-level building classification used during planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildingArchetype {
    Residential,
    Commercial,
    Industrial,
}

/// Simple façade/material hints that the renderer can map onto palettes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FacadeStyle {
    Brick,
    Concrete,
    Glass,
    Metal,
    Painted,
}

/// Planned building geometry and look.
#[derive(Clone, Debug)]
pub struct BuildingPlan {
    pub lot_index: usize,
    pub building_type: BuildingArchetype,
    pub zone: ZoneType,
    pub density: DensityTier,
    pub center: Vec2,
    pub footprint: Vec2,
    pub floors: u32,
    pub floor_height: f32,
    pub height: f32,
    pub shape: BuildingShape,
    pub facade: FacadeStyle,
}

/// Planned park footprint.
#[derive(Clone, Debug)]
pub struct ParkPlan {
    pub lot_index: usize,
    pub center: Vec2,
    pub size: Vec2,
}

/// Shape to use when spawning the building mesh.
#[derive(Clone, Copy, Debug)]
pub enum BuildingShape {
    Box,
    LShape,
    TowerOnBase,
    Stepped,
}

/// Collection of planned structures ready for spawning.
#[derive(Resource, Default)]
pub struct BuildingBlueprints {
    pub plans: Vec<PlannedStructure>,
    pub generated: bool,
}

#[derive(Clone, Debug)]
pub enum PlannedStructure {
    Building(BuildingPlan),
    Park(ParkPlan),
}

fn should_plan_blueprints(plans: Res<LotPlans>, blueprints: Res<BuildingBlueprints>) -> bool {
    plans.generated && !blueprints.generated
}

fn plan_blueprints(
    planned_lots: Res<LotPlans>,
    mut blueprints: ResMut<BuildingBlueprints>,
    config: Res<BuildingFactoryConfig>,
) {
    info!(
        "Planning building blueprints for {} lots",
        planned_lots.planned.len()
    );

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut results = Vec::new();

    for (lot_index, planned) in planned_lots.planned.iter().enumerate() {
        if let Some(structure) = plan_structure(lot_index, planned, &mut rng, &config) {
            results.push(structure);
        }
    }

    blueprints.plans = results;
    blueprints.generated = true;
}

fn plan_structure(
    lot_index: usize,
    planned: &PlannedLot,
    rng: &mut StdRng,
    config: &BuildingFactoryConfig,
) -> Option<PlannedStructure> {
    // Green lots become parks immediately.
    if planned.zone == ZoneType::Green {
        let shrunk = shrink_polygon(&planned.lot.vertices, config.setback * 0.5);
        if shrunk.len() < 3 {
            return None;
        }
        let (min, max) = polygon_bounds(&shrunk);
        let center = (min + max) / 2.0;
        let size = max - min;

        return Some(PlannedStructure::Park(ParkPlan {
            lot_index,
            center,
            size,
        }));
    }

    let shrunk = shrink_polygon(&planned.lot.vertices, config.setback);
    if shrunk.len() < 3 {
        return None;
    }

    let (min, max) = polygon_bounds(&shrunk);
    let mut size = max - min;
    if size.x < config.min_footprint || size.y < config.min_footprint {
        return None;
    }

    // Slightly vary footprint coverage to avoid uniformity.
    let coverage = 1.0 - rng.gen_range(0.0..config.coverage_variance);
    size *= coverage.max(0.6);

    let center = (min + max) / 2.0;
    let building_type = map_zone_to_archetype(planned.zone);
    let floor_height = match building_type {
        BuildingArchetype::Residential => 3.0,
        BuildingArchetype::Commercial => 3.75,
        BuildingArchetype::Industrial => 3.5,
    };

    let floors = choose_floors(
        building_type,
        planned.density,
        size,
        planned.lot.area,
        config.large_lot_area,
        rng,
    );
    let height = floor_height * floors as f32;
    let shape = pick_shape(building_type, planned.density, size, rng);
    let facade = pick_facade(
        building_type,
        planned.density,
        planned.environment.sunlight,
        rng,
    );

    Some(PlannedStructure::Building(BuildingPlan {
        lot_index,
        building_type,
        zone: planned.zone,
        density: planned.density,
        center,
        footprint: size,
        floors,
        floor_height,
        height,
        shape,
        facade,
    }))
}

fn map_zone_to_archetype(zone: ZoneType) -> BuildingArchetype {
    match zone {
        ZoneType::Residential => BuildingArchetype::Residential,
        ZoneType::Commercial | ZoneType::Civic => BuildingArchetype::Commercial,
        ZoneType::Industrial => BuildingArchetype::Industrial,
        ZoneType::Green => BuildingArchetype::Residential,
    }
}

fn choose_floors(
    archetype: BuildingArchetype,
    density: DensityTier,
    footprint: Vec2,
    lot_area: f32,
    large_lot_area: f32,
    rng: &mut StdRng,
) -> u32 {
    let (min, base_max) = match density {
        DensityTier::Low => (1, 4),
        DensityTier::Medium => (3, 9),
        DensityTier::High => match archetype {
            BuildingArchetype::Commercial => (8, 22),
            BuildingArchetype::Industrial => (4, 12),
            BuildingArchetype::Residential => (6, 14),
        },
    };

    // Scale potential height by lot size to prefer taller buildings on larger lots.
    let footprint_area = footprint.x * footprint.y;
    let area_factor = (footprint_area / large_lot_area).clamp(0.6, 2.5);
    let max = ((base_max as f32) * area_factor).ceil() as u32;
    let mut floors = rng.gen_range(min..=max.max(min));

    // Small lots should top out sooner.
    if lot_area < large_lot_area * 0.6 {
        floors = floors.saturating_sub(1);
    }

    floors.max(min)
}

fn pick_shape(
    archetype: BuildingArchetype,
    density: DensityTier,
    size: Vec2,
    rng: &mut StdRng,
) -> BuildingShape {
    let roll = rng.gen::<f32>();
    match archetype {
        BuildingArchetype::Commercial => {
            if density == DensityTier::High && roll < 0.5 {
                BuildingShape::TowerOnBase
            } else if roll < 0.7 {
                BuildingShape::Stepped
            } else if roll < 0.85 && size.x > 6.0 && size.y > 6.0 {
                BuildingShape::LShape
            } else {
                BuildingShape::Box
            }
        }
        BuildingArchetype::Residential => {
            if roll < 0.25 && size.x > 6.0 && size.y > 6.0 {
                BuildingShape::LShape
            } else if roll < 0.45 {
                BuildingShape::Stepped
            } else {
                BuildingShape::Box
            }
        }
        BuildingArchetype::Industrial => {
            if roll < 0.2 && size.x > 8.0 && size.y > 8.0 {
                BuildingShape::LShape
            } else {
                BuildingShape::Box
            }
        }
    }
}

fn pick_facade(
    archetype: BuildingArchetype,
    density: DensityTier,
    sunlight: f32,
    rng: &mut StdRng,
) -> FacadeStyle {
    let roll = rng.gen::<f32>();
    match archetype {
        BuildingArchetype::Residential => {
            if roll < 0.45 {
                FacadeStyle::Brick
            } else if sunlight > 0.6 && roll < 0.75 {
                FacadeStyle::Painted
            } else {
                FacadeStyle::Concrete
            }
        }
        BuildingArchetype::Commercial => {
            if density == DensityTier::High && roll < 0.55 {
                FacadeStyle::Glass
            } else if roll < 0.75 {
                FacadeStyle::Concrete
            } else {
                FacadeStyle::Metal
            }
        }
        BuildingArchetype::Industrial => {
            if roll < 0.6 {
                FacadeStyle::Metal
            } else {
                FacadeStyle::Concrete
            }
        }
    }
}

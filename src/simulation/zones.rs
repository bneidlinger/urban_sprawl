//! Zone growth system - spawns buildings in zoned areas based on demand.
//!
//! Growth is influenced by:
//! - RCI demand for the zone type
//! - Land value (environmental factors, services, pollution, crime)
//! - Service coverage
//!
//! When conditions are met, a construction site is spawned first. The building
//! appears when construction completes.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::game_state::GameState;
use crate::procgen::lot_engine::ZoneType;
use crate::render::construction_sites::{spawn_construction_site, ConstructionConfig};
use crate::tools::zone_paint::{ZoneCell, ZonePaintConfig};

use super::demand::RCIDemand;
use super::land_value::ZoneFactors;

pub struct ZoneGrowthPlugin;

impl Plugin for ZoneGrowthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneGrowthConfig>()
            .add_systems(
                Update,
                process_zone_growth.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Configuration for zone growth.
#[derive(Resource)]
pub struct ZoneGrowthConfig {
    /// How often to check for growth opportunities (seconds).
    pub growth_tick_interval: f32,
    /// Base probability of growth when demand is 1.0.
    pub base_growth_chance: f32,
    /// Minimum demand to allow growth.
    pub min_demand_threshold: f32,
    /// Random seed for deterministic growth.
    pub seed: u64,
}

impl Default for ZoneGrowthConfig {
    fn default() -> Self {
        Self {
            growth_tick_interval: 1.0,
            base_growth_chance: 0.1, // 10% chance per tick at max demand
            min_demand_threshold: 0.1,
            seed: 42,
        }
    }
}

/// Timer for growth ticks.
#[derive(Resource, Default)]
struct GrowthTimer(f32);

/// Marker for buildings spawned by the growth system.
#[derive(Component)]
pub struct GrownBuilding {
    pub zone_cell: Entity,
    pub growth_time: f32,
}

fn process_zone_growth(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<ZoneGrowthConfig>,
    zone_config: Res<ZonePaintConfig>,
    construction_config: Res<ConstructionConfig>,
    demand: Res<RCIDemand>,
    mut timer: Local<f32>,
    mut rng_seed: Local<u64>,
    mut zone_cells: Query<(Entity, &mut ZoneCell, &Transform, Option<&ZoneFactors>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    *timer += time.delta_secs();

    if *timer < config.growth_tick_interval {
        return;
    }

    *timer = 0.0;
    *rng_seed = rng_seed.wrapping_add(1);
    let mut rng = StdRng::seed_from_u64(config.seed.wrapping_add(*rng_seed));

    let mut constructions_started = 0;

    for (entity, mut cell, transform, factors) in &mut zone_cells {
        // Skip already developed cells (development_level > 0 means building or construction in progress)
        if cell.development_level > 0 {
            continue;
        }

        // Check demand for this zone type
        let zone_demand = demand.for_zone(cell.zone_type);
        if zone_demand < config.min_demand_threshold {
            continue;
        }

        // Get land value modifier (default to 0.5 if not calculated yet)
        let land_value = factors.map(|f| f.0.land_value).unwrap_or(0.5);

        // Calculate growth probability based on demand AND land value
        // High land value = faster growth, low land value = slower growth
        let land_value_modifier = 0.5 + land_value; // Range: 0.5 to 1.5
        let growth_chance = config.base_growth_chance * zone_demand * land_value_modifier;

        if rng.gen::<f32>() > growth_chance {
            continue;
        }

        // Building height influenced by land value
        // Higher land value = taller buildings (more valuable to develop)
        let height_multiplier = 0.7 + land_value * 0.6; // Range: 0.7 to 1.3

        let building_height = match cell.zone_type {
            ZoneType::Residential => rng.gen_range(8.0..20.0) * height_multiplier,
            ZoneType::Commercial => rng.gen_range(15.0..40.0) * height_multiplier,
            ZoneType::Industrial => rng.gen_range(6.0..15.0) * height_multiplier,
            _ => continue,
        };

        let building_size = zone_config.cell_size * 0.8;

        // Spawn construction site instead of building directly
        let site_pos = Vec3::new(transform.translation.x, 0.0, transform.translation.z);

        let _site_entity = spawn_construction_site(
            &mut commands,
            &mut meshes,
            &mut materials,
            site_pos,
            building_height,
            building_size,
            cell.zone_type,
            entity,
            &construction_config,
            &mut rng,
        );

        // Mark cell as under development (building entity will be set when construction completes)
        cell.development_level = 1;
        constructions_started += 1;
    }

    if constructions_started > 0 {
        info!("Zone growth: {} construction sites started", constructions_started);
    }
}

fn building_color(zone_type: ZoneType) -> Color {
    match zone_type {
        ZoneType::Residential => Color::srgb(0.85, 0.9, 0.85), // Light green-white
        ZoneType::Commercial => Color::srgb(0.7, 0.75, 0.9),   // Light blue
        ZoneType::Industrial => Color::srgb(0.8, 0.75, 0.6),   // Tan/beige
        ZoneType::Civic => Color::srgb(0.9, 0.85, 0.95),       // Light purple
        ZoneType::Green => Color::srgb(0.3, 0.7, 0.3),         // Green
    }
}

fn zone_to_building_type(
    zone_type: ZoneType,
) -> crate::procgen::building_factory::BuildingArchetype {
    match zone_type {
        ZoneType::Residential => crate::procgen::building_factory::BuildingArchetype::Residential,
        ZoneType::Commercial => crate::procgen::building_factory::BuildingArchetype::Commercial,
        ZoneType::Industrial => crate::procgen::building_factory::BuildingArchetype::Industrial,
        // Default to commercial for civic/green
        _ => crate::procgen::building_factory::BuildingArchetype::Commercial,
    }
}

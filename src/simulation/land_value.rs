//! Land value calculation based on multiple environmental and service factors.
//!
//! Computes a composite land value score for each location based on:
//! - Pollution (negative, from industrial buildings)
//! - Crime rate (negative, reduced by police coverage)
//! - Education access (positive, from schools)
//! - Healthcare access (positive, from hospitals)
//! - Park access (positive, from parks/green spaces)
//! - Road access (positive/negative based on zone type)
//! - Commute time (negative, distance to jobs)

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::Building;
use crate::tools::services::{ServiceBuilding, ServiceType};
use crate::tools::zone_paint::ZoneCell;
use crate::tools::ZoneType;

pub struct LandValuePlugin;

impl Plugin for LandValuePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LandValueConfig>()
            .init_resource::<LandValueMap>()
            .add_systems(
                Update,
                (update_land_value_map, apply_land_value_to_zones)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Configuration for land value calculations.
#[derive(Resource)]
pub struct LandValueConfig {
    /// How often to recalculate land values (seconds).
    pub update_interval: f32,
    /// Maximum distance for pollution spread.
    pub pollution_radius: f32,
    /// Base crime level without police coverage.
    pub base_crime: f32,
    /// Maximum distance to consider for commute.
    pub max_commute_distance: f32,
    /// Grid cell size for land value sampling.
    pub grid_size: f32,
}

impl Default for LandValueConfig {
    fn default() -> Self {
        Self {
            update_interval: 2.0,
            pollution_radius: 80.0,
            base_crime: 0.5,
            max_commute_distance: 200.0,
            grid_size: 20.0,
        }
    }
}

/// Comprehensive environmental factors for a location.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocationFactors {
    /// Pollution level (0.0 = clean, 1.0 = heavily polluted).
    pub pollution: f32,
    /// Crime rate (0.0 = safe, 1.0 = high crime).
    pub crime: f32,
    /// Education access (0.0 = no access, 1.0 = excellent).
    pub education: f32,
    /// Healthcare access (0.0 = no access, 1.0 = excellent).
    pub healthcare: f32,
    /// Fire safety (0.0 = no coverage, 1.0 = full coverage).
    pub fire_safety: f32,
    /// Park/greenery access (0.0 = none, 1.0 = excellent).
    pub park_access: f32,
    /// Road accessibility (0.0 = none, 1.0 = excellent).
    pub road_access: f32,
    /// Commute time factor (0.0 = long commute, 1.0 = short commute).
    pub commute: f32,
    /// Composite land value (0.0 = undesirable, 1.0 = prime location).
    pub land_value: f32,
}

impl LocationFactors {
    /// Calculate composite land value from individual factors.
    pub fn calculate_land_value(&mut self, zone_type: Option<ZoneType>) {
        // Weights vary by zone type
        let (pollution_weight, crime_weight, edu_weight, health_weight, park_weight, commute_weight) =
            match zone_type {
                Some(ZoneType::Residential) => (-0.25, -0.20, 0.15, 0.10, 0.15, 0.15),
                Some(ZoneType::Commercial) => (-0.10, -0.25, 0.05, 0.05, 0.10, 0.20),
                Some(ZoneType::Industrial) => (0.0, -0.15, 0.0, 0.0, 0.0, 0.10),
                _ => (-0.15, -0.15, 0.10, 0.10, 0.10, 0.10),
            };

        // Base value starts at 0.5
        let mut value = 0.5;

        // Apply weighted factors
        value += self.pollution * pollution_weight;
        value += self.crime * crime_weight;
        value += self.education * edu_weight;
        value += self.healthcare * health_weight;
        value += self.park_access * park_weight;
        value += self.commute * commute_weight;
        value += self.fire_safety * 0.05;
        value += self.road_access * 0.10;

        self.land_value = value.clamp(0.0, 1.0);
    }
}

/// Cached land value data for the city.
#[derive(Resource, Default)]
pub struct LandValueMap {
    /// Timer for periodic updates.
    pub update_timer: f32,
    /// Whether the map needs recalculation.
    pub dirty: bool,
}

/// Component to store calculated factors for a zone cell.
#[derive(Component, Default)]
pub struct ZoneFactors(pub LocationFactors);

fn update_land_value_map(
    time: Res<Time>,
    config: Res<LandValueConfig>,
    mut map: ResMut<LandValueMap>,
) {
    map.update_timer += time.delta_secs();

    if map.update_timer >= config.update_interval {
        map.update_timer = 0.0;
        map.dirty = true;
    }
}

fn apply_land_value_to_zones(
    mut commands: Commands,
    config: Res<LandValueConfig>,
    mut map: ResMut<LandValueMap>,
    zone_cells: Query<(Entity, &ZoneCell, &Transform), Without<ZoneFactors>>,
    mut existing_zones: Query<(Entity, &ZoneCell, &Transform, &mut ZoneFactors)>,
    buildings: Query<(&Building, &GlobalTransform)>,
    services: Query<(&ServiceBuilding, &GlobalTransform)>,
) {
    // Add ZoneFactors to cells that don't have them
    for (entity, cell, transform) in &zone_cells {
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        let factors = calculate_factors_at(pos, cell.zone_type, &config, &buildings, &services);
        commands.entity(entity).insert(ZoneFactors(factors));
    }

    // Update existing zones periodically
    if !map.dirty {
        return;
    }
    map.dirty = false;

    for (_, cell, transform, mut factors) in &mut existing_zones {
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        factors.0 = calculate_factors_at(pos, cell.zone_type, &config, &buildings, &services);
    }
}

fn calculate_factors_at(
    pos: Vec2,
    zone_type: ZoneType,
    config: &LandValueConfig,
    buildings: &Query<(&Building, &GlobalTransform)>,
    services: &Query<(&ServiceBuilding, &GlobalTransform)>,
) -> LocationFactors {
    let mut factors = LocationFactors::default();

    // Calculate pollution from industrial buildings
    let mut pollution = 0.0f32;
    let mut job_distance = f32::MAX;

    for (building, transform) in buildings {
        let building_pos = Vec2::new(transform.translation().x, transform.translation().z);
        let distance = pos.distance(building_pos);

        match building.building_type {
            BuildingArchetype::Industrial => {
                // Industrial buildings cause pollution
                if distance < config.pollution_radius {
                    let intensity = 1.0 - (distance / config.pollution_radius);
                    pollution += intensity * 0.3;
                }
                // Industrial provides jobs
                job_distance = job_distance.min(distance);
            }
            BuildingArchetype::Commercial => {
                // Commercial provides jobs too
                job_distance = job_distance.min(distance);
            }
            BuildingArchetype::Residential => {
                // Residential doesn't affect pollution or jobs
            }
        }
    }
    factors.pollution = pollution.clamp(0.0, 1.0);

    // Calculate commute factor (closer to jobs = better)
    if job_distance < f32::MAX {
        factors.commute = 1.0 - (job_distance / config.max_commute_distance).clamp(0.0, 1.0);
    } else {
        factors.commute = 0.5; // Neutral if no jobs exist yet
    }

    // Calculate service coverage
    let mut has_police = false;
    let mut has_fire = false;
    let mut has_hospital = false;
    let mut has_school = false;
    let mut park_access = 0.0f32;

    for (service, transform) in services {
        let service_pos = Vec2::new(transform.translation().x, transform.translation().z);
        let distance = pos.distance(service_pos);

        if distance <= service.radius {
            let coverage = 1.0 - (distance / service.radius);

            match service.service_type {
                ServiceType::Police => {
                    has_police = true;
                    factors.crime = (config.base_crime * (1.0 - coverage * 0.8)).max(0.0);
                }
                ServiceType::Fire => {
                    has_fire = true;
                    factors.fire_safety = factors.fire_safety.max(coverage);
                }
                ServiceType::Hospital => {
                    has_hospital = true;
                    factors.healthcare = factors.healthcare.max(coverage);
                }
                ServiceType::School => {
                    has_school = true;
                    factors.education = factors.education.max(coverage);
                }
                ServiceType::Park => {
                    park_access = park_access.max(coverage);
                }
            }
        }
    }

    // Apply defaults if no coverage
    if !has_police {
        factors.crime = config.base_crime;
    }
    if !has_fire {
        factors.fire_safety = 0.0;
    }
    if !has_hospital {
        factors.healthcare = 0.0;
    }
    if !has_school {
        factors.education = 0.0;
    }
    factors.park_access = park_access;

    // Road access (simplified - assume zones near roads have good access)
    // This would ideally check actual road graph distance
    factors.road_access = 0.7; // Default moderate access for zoned areas

    // Calculate final land value
    factors.calculate_land_value(Some(zone_type));

    factors
}

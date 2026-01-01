//! Service coverage calculation and effects.
//!
//! Calculates how well services cover the city and affects:
//! - Population happiness/growth
//! - Crime rates
//! - Fire damage risk
//! - Health outcomes
//! - Education levels

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::render::building_spawner::Building;
use crate::tools::services::{ServiceBuilding, ServiceType};

pub struct ServiceCoveragePlugin;

impl Plugin for ServiceCoveragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityCoverage>()
            .init_resource::<ServiceEffects>()
            .add_systems(
                Update,
                (calculate_city_coverage, apply_service_effects)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Overall city service coverage statistics.
#[derive(Resource, Default)]
pub struct CityCoverage {
    /// Percentage of residential buildings covered by police (0-100).
    pub police_coverage: f32,
    /// Percentage of buildings covered by fire department (0-100).
    pub fire_coverage: f32,
    /// Percentage of residential buildings with hospital access (0-100).
    pub healthcare_coverage: f32,
    /// Percentage of residential buildings with school access (0-100).
    pub education_coverage: f32,
    /// Average park access score (0-100).
    pub park_access: f32,
    /// Timer for recalculation.
    update_timer: f32,
}

impl CityCoverage {
    /// Overall service score (0-100).
    pub fn overall_score(&self) -> f32 {
        (self.police_coverage * 0.2
            + self.fire_coverage * 0.15
            + self.healthcare_coverage * 0.25
            + self.education_coverage * 0.25
            + self.park_access * 0.15)
            .clamp(0.0, 100.0)
    }
}

/// Effects that services have on the city.
#[derive(Resource)]
pub struct ServiceEffects {
    /// Crime rate modifier (0.0 = no crime, 1.0 = high crime).
    pub crime_modifier: f32,
    /// Fire risk modifier (0.0 = no risk, 1.0 = high risk).
    pub fire_risk: f32,
    /// Health modifier affecting population growth.
    pub health_modifier: f32,
    /// Education modifier affecting commercial demand.
    pub education_modifier: f32,
    /// Happiness modifier from parks.
    pub happiness_modifier: f32,
}

impl Default for ServiceEffects {
    fn default() -> Self {
        Self {
            crime_modifier: 0.5,
            fire_risk: 0.3,
            health_modifier: 0.0,
            education_modifier: 0.0,
            happiness_modifier: 0.0,
        }
    }
}

fn calculate_city_coverage(
    time: Res<Time>,
    mut coverage: ResMut<CityCoverage>,
    buildings: Query<(&Building, &GlobalTransform)>,
    services: Query<(&ServiceBuilding, &GlobalTransform)>,
) {
    coverage.update_timer += time.delta_secs();

    // Only recalculate every 2 seconds
    if coverage.update_timer < 2.0 {
        return;
    }
    coverage.update_timer = 0.0;

    // Collect building positions
    let building_positions: Vec<(Vec2, bool)> = buildings
        .iter()
        .map(|(b, t)| {
            let pos = Vec2::new(t.translation().x, t.translation().z);
            let is_residential = matches!(
                b.building_type,
                crate::procgen::building_factory::BuildingArchetype::Residential
            );
            (pos, is_residential)
        })
        .collect();

    if building_positions.is_empty() {
        // No buildings, reset to defaults
        coverage.police_coverage = 0.0;
        coverage.fire_coverage = 0.0;
        coverage.healthcare_coverage = 0.0;
        coverage.education_coverage = 0.0;
        coverage.park_access = 0.0;
        return;
    }

    // Collect service positions and radii
    let service_list: Vec<(ServiceType, Vec2, f32)> = services
        .iter()
        .map(|(s, t)| {
            let pos = Vec2::new(t.translation().x, t.translation().z);
            (s.service_type, pos, s.radius)
        })
        .collect();

    // Calculate coverage for each building
    let mut police_covered = 0;
    let mut fire_covered = 0;
    let mut health_covered = 0;
    let mut edu_covered = 0;
    let mut total_park_access = 0.0f32;
    let mut residential_count = 0;
    let total_buildings = building_positions.len();

    for (pos, is_residential) in &building_positions {
        if *is_residential {
            residential_count += 1;
        }

        for (service_type, service_pos, radius) in &service_list {
            let distance = pos.distance(*service_pos);
            if distance <= *radius {
                match service_type {
                    ServiceType::Police => {
                        if *is_residential {
                            police_covered += 1;
                        }
                    }
                    ServiceType::Fire => {
                        fire_covered += 1;
                    }
                    ServiceType::Hospital => {
                        if *is_residential {
                            health_covered += 1;
                        }
                    }
                    ServiceType::School => {
                        if *is_residential {
                            edu_covered += 1;
                        }
                    }
                    ServiceType::Park => {
                        let access = 1.0 - (distance / radius);
                        total_park_access += access;
                    }
                }
            }
        }
    }

    // Calculate percentages
    let res_count = residential_count.max(1) as f32;
    let total = total_buildings.max(1) as f32;

    coverage.police_coverage = (police_covered as f32 / res_count * 100.0).min(100.0);
    coverage.fire_coverage = (fire_covered as f32 / total * 100.0).min(100.0);
    coverage.healthcare_coverage = (health_covered as f32 / res_count * 100.0).min(100.0);
    coverage.education_coverage = (edu_covered as f32 / res_count * 100.0).min(100.0);
    coverage.park_access = (total_park_access / total * 100.0).min(100.0);
}

fn apply_service_effects(coverage: Res<CityCoverage>, mut effects: ResMut<ServiceEffects>) {
    if !coverage.is_changed() {
        return;
    }

    // Crime is reduced by police coverage
    // 0% coverage = 0.8 crime, 100% coverage = 0.1 crime
    effects.crime_modifier = 0.8 - (coverage.police_coverage / 100.0 * 0.7);

    // Fire risk is reduced by fire coverage
    // 0% coverage = 0.5 risk, 100% coverage = 0.05 risk
    effects.fire_risk = 0.5 - (coverage.fire_coverage / 100.0 * 0.45);

    // Health affects population growth rate
    // 0% coverage = -0.1, 100% coverage = +0.2
    effects.health_modifier = (coverage.healthcare_coverage / 100.0 * 0.3) - 0.1;

    // Education affects commercial/office demand
    // 0% coverage = -0.1, 100% coverage = +0.2
    effects.education_modifier = (coverage.education_coverage / 100.0 * 0.3) - 0.1;

    // Parks affect happiness
    // 0% coverage = 0.0, high coverage = +0.3
    effects.happiness_modifier = (coverage.park_access / 100.0 * 0.3).min(0.3);
}

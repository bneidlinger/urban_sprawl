//! RCI (Residential/Commercial/Industrial) demand system.
//!
//! Calculates demand for each zone type based on:
//! - Population vs housing capacity (R demand)
//! - Population vs jobs (C/I demand)
//! - Zone balance

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::tools::zone_paint::ZoneCell;
use crate::tools::ZoneType;

pub struct DemandPlugin;

impl Plugin for DemandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RCIDemand>()
            .init_resource::<CityStats>()
            .add_systems(
                Update,
                (update_city_stats, calculate_demand)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Demand levels for each zone type.
/// Range: -1.0 (oversupply) to 1.0 (high demand)
#[derive(Resource, Default)]
pub struct RCIDemand {
    pub residential: f32,
    pub commercial: f32,
    pub industrial: f32,
}

impl RCIDemand {
    /// Get demand for a specific zone type.
    pub fn for_zone(&self, zone_type: ZoneType) -> f32 {
        match zone_type {
            ZoneType::Residential => self.residential,
            ZoneType::Commercial => self.commercial,
            ZoneType::Industrial => self.industrial,
            ZoneType::Civic | ZoneType::Green => 0.0,
        }
    }
}

/// Aggregate city statistics used for demand calculation.
#[derive(Resource, Default)]
pub struct CityStats {
    /// Total population.
    pub population: u32,
    /// Housing capacity (residential buildings).
    pub housing_capacity: u32,
    /// Commercial jobs available.
    pub commercial_jobs: u32,
    /// Industrial jobs available.
    pub industrial_jobs: u32,
    /// Number of each zone type.
    pub residential_zones: u32,
    pub commercial_zones: u32,
    pub industrial_zones: u32,
    /// Developed zones (have buildings).
    pub developed_residential: u32,
    pub developed_commercial: u32,
    pub developed_industrial: u32,
}

impl CityStats {
    pub fn total_jobs(&self) -> u32 {
        self.commercial_jobs + self.industrial_jobs
    }

    pub fn employment_rate(&self) -> f32 {
        if self.population == 0 {
            return 1.0;
        }
        let working_pop = (self.population as f32 * 0.6) as u32; // 60% working age
        if working_pop == 0 {
            return 1.0;
        }
        (self.total_jobs() as f32 / working_pop as f32).min(1.0)
    }
}

fn update_city_stats(
    mut stats: ResMut<CityStats>,
    zone_cells: Query<&ZoneCell>,
    buildings: Query<&crate::render::building_spawner::Building>,
    population: Res<super::population::Population>,
) {
    // Update population from Population resource
    stats.population = population.total;
    // Count zones
    let mut res_zones = 0u32;
    let mut com_zones = 0u32;
    let mut ind_zones = 0u32;
    let mut dev_res = 0u32;
    let mut dev_com = 0u32;
    let mut dev_ind = 0u32;

    for cell in &zone_cells {
        match cell.zone_type {
            ZoneType::Residential => {
                res_zones += 1;
                if cell.development_level > 0 {
                    dev_res += 1;
                }
            }
            ZoneType::Commercial => {
                com_zones += 1;
                if cell.development_level > 0 {
                    dev_com += 1;
                }
            }
            ZoneType::Industrial => {
                ind_zones += 1;
                if cell.development_level > 0 {
                    dev_ind += 1;
                }
            }
            _ => {}
        }
    }

    // Count buildings and calculate capacity/jobs
    let mut housing = 0u32;
    let mut com_jobs = 0u32;
    let mut ind_jobs = 0u32;

    for building in &buildings {
        match building.building_type {
            crate::procgen::building_factory::BuildingArchetype::Residential => {
                // Each residential building provides housing for ~10-50 people
                housing += 20;
            }
            crate::procgen::building_factory::BuildingArchetype::Commercial => {
                // Each commercial building provides ~5-20 jobs
                com_jobs += 10;
            }
            crate::procgen::building_factory::BuildingArchetype::Industrial => {
                // Each industrial building provides ~10-30 jobs
                ind_jobs += 15;
            }
        }
    }

    stats.residential_zones = res_zones;
    stats.commercial_zones = com_zones;
    stats.industrial_zones = ind_zones;
    stats.developed_residential = dev_res;
    stats.developed_commercial = dev_com;
    stats.developed_industrial = dev_ind;
    stats.housing_capacity = housing;
    stats.commercial_jobs = com_jobs;
    stats.industrial_jobs = ind_jobs;
}

fn calculate_demand(stats: Res<CityStats>, mut demand: ResMut<RCIDemand>) {
    // Base demand starts neutral
    let mut r_demand = 0.0f32;
    let mut c_demand = 0.0f32;
    let mut i_demand = 0.0f32;

    // If no zones at all, high demand for everything
    if stats.residential_zones == 0 && stats.commercial_zones == 0 && stats.industrial_zones == 0 {
        demand.residential = 0.8;
        demand.commercial = 0.5;
        demand.industrial = 0.3;
        return;
    }

    // Residential demand based on jobs vs housing
    // More jobs than housing = people want to move in
    let total_jobs = stats.total_jobs();
    if stats.housing_capacity > 0 {
        let job_housing_ratio = total_jobs as f32 / stats.housing_capacity as f32;
        r_demand = (job_housing_ratio - 0.8).clamp(-1.0, 1.0);
    } else if total_jobs > 0 {
        r_demand = 1.0; // Jobs but no housing = high demand
    } else {
        r_demand = 0.5; // No jobs, no housing = moderate demand to kickstart
    }

    // Commercial demand based on population
    // More population = more commercial demand
    if stats.population > 0 {
        let pop_per_commercial = if stats.commercial_jobs > 0 {
            stats.population as f32 / stats.commercial_jobs as f32
        } else {
            100.0
        };
        c_demand = ((pop_per_commercial - 2.0) / 5.0).clamp(-1.0, 1.0);
    } else if stats.residential_zones > 0 {
        // Residential zones but no population yet = moderate commercial demand
        c_demand = 0.3;
    }

    // Industrial demand based on commercial/residential balance
    // Need industry to provide goods for commercial
    if stats.commercial_zones > 0 {
        let ind_com_ratio = if stats.industrial_zones > 0 {
            stats.industrial_zones as f32 / stats.commercial_zones as f32
        } else {
            0.0
        };
        i_demand = ((0.5 - ind_com_ratio) * 2.0).clamp(-1.0, 1.0);
    } else if stats.residential_zones > 0 {
        i_demand = 0.2; // Some industrial demand even without commercial
    }

    // Boost demand for undeveloped zones
    let res_dev_rate = if stats.residential_zones > 0 {
        stats.developed_residential as f32 / stats.residential_zones as f32
    } else {
        1.0
    };
    let com_dev_rate = if stats.commercial_zones > 0 {
        stats.developed_commercial as f32 / stats.commercial_zones as f32
    } else {
        1.0
    };
    let ind_dev_rate = if stats.industrial_zones > 0 {
        stats.developed_industrial as f32 / stats.industrial_zones as f32
    } else {
        1.0
    };

    // Higher demand if zones exist but aren't developed
    if res_dev_rate < 0.5 {
        r_demand = r_demand.max(0.3);
    }
    if com_dev_rate < 0.5 {
        c_demand = c_demand.max(0.2);
    }
    if ind_dev_rate < 0.5 {
        i_demand = i_demand.max(0.1);
    }

    demand.residential = r_demand;
    demand.commercial = c_demand;
    demand.industrial = i_demand;
}

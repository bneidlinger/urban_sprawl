//! Population tracking and growth system.
//!
//! Population growth is influenced by:
//! - Housing availability
//! - Employment rate
//! - City finances
//! - Service coverage (health, education, parks)
//! - Commute quality

use bevy::prelude::*;

use crate::game_state::GameState;

use super::commute::CommuteStats;
use super::demand::CityStats;
use super::economy::CityBudget;
use super::services::ServiceEffects;

pub struct PopulationPlugin;

impl Plugin for PopulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Population>()
            .init_resource::<PopulationConfig>()
            .add_systems(
                Update,
                update_population.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Population configuration.
#[derive(Resource)]
pub struct PopulationConfig {
    /// Base growth rate per tick when conditions are good.
    pub base_growth_rate: f32,
    /// How often to update population (seconds).
    pub update_interval: f32,
    /// Minimum population to start with.
    pub starting_population: u32,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            base_growth_rate: 0.02, // 2% growth per update
            update_interval: 2.0,
            starting_population: 0,
        }
    }
}

/// City population tracking.
#[derive(Resource)]
pub struct Population {
    /// Total population.
    pub total: u32,
    /// Population change since last update.
    pub change: i32,
    /// Growth rate (can be negative).
    pub growth_rate: f32,
    /// Timer for updates.
    pub update_timer: f32,
    /// Historical population (for graphs).
    pub history: Vec<u32>,
}

impl Default for Population {
    fn default() -> Self {
        Self {
            total: 0,
            change: 0,
            growth_rate: 0.0,
            update_timer: 0.0,
            history: Vec::new(),
        }
    }
}

fn update_population(
    time: Res<Time>,
    config: Res<PopulationConfig>,
    mut population: ResMut<Population>,
    stats: Res<CityStats>,
    budget: Res<CityBudget>,
    service_effects: Res<ServiceEffects>,
    commute_stats: Res<CommuteStats>,
) {
    population.update_timer += time.delta_secs();

    if population.update_timer < config.update_interval {
        return;
    }

    population.update_timer = 0.0;

    let old_pop = population.total;

    // Calculate growth factors
    let mut growth_modifier = 1.0f32;

    // Housing availability affects growth
    if stats.housing_capacity > 0 {
        let occupancy = population.total as f32 / stats.housing_capacity as f32;
        if occupancy > 0.95 {
            // Near capacity - slow/stop growth
            growth_modifier *= 0.1;
        } else if occupancy > 0.8 {
            // Getting full
            growth_modifier *= 0.5;
        } else if occupancy < 0.3 {
            // Lots of room - faster growth
            growth_modifier *= 1.5;
        }
    } else if population.total > 0 {
        // No housing but have people - decline
        growth_modifier = -0.5;
    }

    // Jobs affect growth
    let employment = stats.employment_rate();
    if employment < 0.5 {
        // High unemployment - people leave
        growth_modifier *= 0.5;
    } else if employment > 0.9 {
        // Good jobs - attracts people
        growth_modifier *= 1.2;
    }

    // City finances affect growth
    if budget.funds < 0 {
        growth_modifier *= 0.3; // Broke city is unattractive
    } else if budget.net_flow > 0 {
        growth_modifier *= 1.1; // Prosperous city attracts people
    }

    // Service effects modify growth
    // Health: -0.1 to +0.2 modifier
    growth_modifier *= 1.0 + service_effects.health_modifier;

    // Happiness from parks: 0 to +0.3 modifier
    growth_modifier *= 1.0 + service_effects.happiness_modifier * 0.5;

    // Crime reduces attractiveness: 0.8 crime = 20% penalty
    growth_modifier *= 1.0 - (service_effects.crime_modifier * 0.25);

    // Commute quality affects attractiveness
    // Good commute (score 80+) = bonus, bad commute (score < 40) = penalty
    if commute_stats.commute_score > 80.0 {
        growth_modifier *= 1.1;
    } else if commute_stats.commute_score < 40.0 {
        growth_modifier *= 0.85;
    }

    // Calculate actual growth
    let base_growth = if population.total > 0 {
        (population.total as f32 * config.base_growth_rate * growth_modifier) as i32
    } else if stats.housing_capacity > 0 {
        // Initial population - some people move in to available housing
        (stats.housing_capacity as f32 * 0.1).max(1.0) as i32
    } else {
        0
    };

    // Apply growth (minimum 0 population)
    let new_pop = (population.total as i32 + base_growth).max(0) as u32;

    // Cap at housing capacity
    let new_pop = new_pop.min(stats.housing_capacity.max(population.total));

    population.change = new_pop as i32 - old_pop as i32;
    population.total = new_pop;
    population.growth_rate = if old_pop > 0 {
        population.change as f32 / old_pop as f32
    } else {
        0.0
    };

    // Record history (keep last 100 entries)
    population.history.push(new_pop);
    if population.history.len() > 100 {
        population.history.remove(0);
    }
}

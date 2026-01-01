//! City economy system - budget, taxes, and costs.

use bevy::prelude::*;

use crate::game_state::GameState;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityBudget>()
            .init_resource::<EconomyConfig>()
            .add_systems(
                Update,
                (calculate_income, calculate_expenses, update_budget)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Configuration for economy calculations.
#[derive(Resource)]
pub struct EconomyConfig {
    /// Starting funds for a new city.
    pub starting_funds: i64,
    /// Tax rate per residential building (per tick).
    pub residential_tax_rate: f32,
    /// Tax rate per commercial building.
    pub commercial_tax_rate: f32,
    /// Tax rate per industrial building.
    pub industrial_tax_rate: f32,
    /// Maintenance cost per road segment.
    pub road_maintenance: f32,
    /// Cost per service building.
    pub service_cost: f32,
    /// How often to process budget (in seconds).
    pub budget_tick_interval: f32,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            starting_funds: 50_000,
            residential_tax_rate: 10.0,
            commercial_tax_rate: 25.0,
            industrial_tax_rate: 20.0,
            road_maintenance: 1.0,
            service_cost: 50.0,
            budget_tick_interval: 1.0, // Every second
        }
    }
}

/// The city's financial state.
#[derive(Resource)]
pub struct CityBudget {
    /// Current available funds.
    pub funds: i64,
    /// Income breakdown for current period.
    pub income: IncomeBreakdown,
    /// Expense breakdown for current period.
    pub expenses: ExpenseBreakdown,
    /// Net change per budget tick.
    pub net_flow: i64,
    /// Timer for budget updates.
    pub tick_timer: f32,
}

impl Default for CityBudget {
    fn default() -> Self {
        Self {
            funds: 50_000,
            income: IncomeBreakdown::default(),
            expenses: ExpenseBreakdown::default(),
            net_flow: 0,
            tick_timer: 0.0,
        }
    }
}

/// Income sources.
#[derive(Default, Clone)]
pub struct IncomeBreakdown {
    pub residential_tax: i64,
    pub commercial_tax: i64,
    pub industrial_tax: i64,
}

impl IncomeBreakdown {
    pub fn total(&self) -> i64 {
        self.residential_tax + self.commercial_tax + self.industrial_tax
    }
}

/// Expense categories.
#[derive(Default, Clone)]
pub struct ExpenseBreakdown {
    pub road_maintenance: i64,
    pub service_costs: i64,
    pub other: i64,
}

impl ExpenseBreakdown {
    pub fn total(&self) -> i64 {
        self.road_maintenance + self.service_costs + self.other
    }
}

/// Marker for buildings that pay taxes.
#[derive(Component)]
pub struct TaxPayer {
    pub tax_amount: f32,
}

/// Marker for things that cost money to maintain.
#[derive(Component)]
pub struct MaintenanceCost {
    pub cost_per_tick: f32,
}

fn calculate_income(
    config: Res<EconomyConfig>,
    mut budget: ResMut<CityBudget>,
    time: Res<Time>,
    buildings: Query<&crate::render::building_spawner::Building>,
) {
    budget.tick_timer += time.delta_secs();

    if budget.tick_timer < config.budget_tick_interval {
        return;
    }

    let mut residential = 0i64;
    let mut commercial = 0i64;
    let mut industrial = 0i64;

    for building in &buildings {
        match building.building_type {
            crate::procgen::building_factory::BuildingArchetype::Residential => {
                residential += config.residential_tax_rate as i64;
            }
            crate::procgen::building_factory::BuildingArchetype::Commercial => {
                commercial += config.commercial_tax_rate as i64;
            }
            crate::procgen::building_factory::BuildingArchetype::Industrial => {
                industrial += config.industrial_tax_rate as i64;
            }
        }
    }

    budget.income = IncomeBreakdown {
        residential_tax: residential,
        commercial_tax: commercial,
        industrial_tax: industrial,
    };
}

fn calculate_expenses(
    config: Res<EconomyConfig>,
    mut budget: ResMut<CityBudget>,
    roads: Res<crate::procgen::roads::RoadGraph>,
) {
    if budget.tick_timer < config.budget_tick_interval {
        return;
    }

    let road_count = roads.edge_count();
    let road_maintenance = (road_count as f32 * config.road_maintenance) as i64;

    budget.expenses = ExpenseBreakdown {
        road_maintenance,
        service_costs: 0, // TODO: Count service buildings
        other: 0,
    };
}

fn update_budget(mut budget: ResMut<CityBudget>, config: Res<EconomyConfig>) {
    if budget.tick_timer < config.budget_tick_interval {
        return;
    }

    // Reset timer
    budget.tick_timer = 0.0;

    // Calculate net flow
    let income = budget.income.total();
    let expenses = budget.expenses.total();
    budget.net_flow = income - expenses;

    // Update funds
    budget.funds += budget.net_flow;

    if budget.net_flow != 0 {
        info!(
            "Budget: ${} (+${} income, -${} expenses = ${})",
            budget.funds,
            income,
            expenses,
            budget.net_flow
        );
    }
}

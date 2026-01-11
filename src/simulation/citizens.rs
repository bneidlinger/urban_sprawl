//! Citizen agent system with individual citizens, homes, jobs, and daily schedules.
//!
//! Citizens are spawned from residential buildings, assigned jobs at commercial/industrial
//! buildings, and follow daily schedules (wake, commute, work, return, sleep).

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::day_night::TimeOfDay;

use super::SimulationTick;

pub struct CitizensPlugin;

impl Plugin for CitizensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CitizenConfig>()
            .init_resource::<CitizenStats>()
            .init_resource::<CitizensSpawned>()
            .add_systems(
                Update,
                (
                    spawn_citizens.run_if(should_spawn_citizens),
                    update_citizen_state,
                    update_citizen_needs,
                    update_citizen_stats,
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct CitizensSpawned(pub bool);

fn should_spawn_citizens(buildings_spawned: Res<BuildingsSpawned>, spawned: Res<CitizensSpawned>) -> bool {
    buildings_spawned.0 && !spawned.0
}

/// Configuration for the citizen simulation.
#[derive(Resource)]
pub struct CitizenConfig {
    pub seed: u64,
    /// Citizens per residential building (average).
    pub citizens_per_residential: f32,
    /// Jobs per commercial building (average).
    pub jobs_per_commercial: f32,
    /// Jobs per industrial building (average).
    pub jobs_per_industrial: f32,
    /// Maximum citizens to simulate.
    pub max_citizens: usize,
    /// Employment rate target (0.0 to 1.0).
    pub target_employment_rate: f32,
    /// Need decay rates per hour.
    pub hunger_decay_rate: f32,
    pub rest_decay_rate: f32,
    pub happiness_decay_rate: f32,
}

impl Default for CitizenConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            citizens_per_residential: 4.0,
            jobs_per_commercial: 6.0,
            jobs_per_industrial: 8.0,
            max_citizens: 1000,
            target_employment_rate: 0.85,
            hunger_decay_rate: 0.04,   // Gets hungry over ~25 hours
            rest_decay_rate: 0.06,     // Gets tired over ~17 hours
            happiness_decay_rate: 0.02, // Slow happiness decay
        }
    }
}

/// Statistics about citizens.
#[derive(Resource, Default)]
pub struct CitizenStats {
    pub total_citizens: usize,
    pub employed: usize,
    pub unemployed: usize,
    pub at_home: usize,
    pub commuting: usize,
    pub at_work: usize,
    pub shopping: usize,
    pub at_leisure: usize,
    pub average_happiness: f32,
    pub average_hunger: f32,
    pub average_rest: f32,
}

/// Citizen needs (Maslow-lite).
#[derive(Clone, Copy, Debug)]
pub struct Needs {
    /// 0.0 = starving, 1.0 = well fed
    pub hunger: f32,
    /// 0.0 = exhausted, 1.0 = well rested
    pub rest: f32,
    /// 0.0 = broke, 1.0 = wealthy
    pub income: f32,
    /// 0.0 = miserable, 1.0 = very happy
    pub happiness: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 0.8,
            rest: 0.8,
            income: 0.5,
            happiness: 0.6,
        }
    }
}

/// Current state of a citizen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CitizenState {
    #[default]
    AtHome,
    Commuting,
    AtWork,
    Shopping,
    Leisure,
    Sleeping,
}

/// Schedule for citizen activities (24-hour cycle).
#[derive(Component, Clone, Debug)]
pub struct DailySchedule {
    pub wake_time: f32,   // 0-24
    pub work_start: f32,
    pub work_end: f32,
    pub sleep_time: f32,
}

impl Default for DailySchedule {
    fn default() -> Self {
        Self {
            wake_time: 7.0,
            work_start: 9.0,
            work_end: 17.0,
            sleep_time: 23.0,
        }
    }
}

/// Citizen component with full agent state.
#[derive(Component)]
pub struct Citizen {
    /// Home building entity.
    pub home: Entity,
    /// Workplace entity (None if unemployed).
    pub work: Option<Entity>,
    /// Current needs.
    pub needs: Needs,
    /// Current activity state.
    pub state: CitizenState,
    /// Age in years.
    pub age: u32,
    /// Time spent in current state (hours).
    pub state_time: f32,
}

impl Default for Citizen {
    fn default() -> Self {
        Self {
            home: Entity::PLACEHOLDER,
            work: None,
            needs: Needs::default(),
            state: CitizenState::AtHome,
            age: 30,
            state_time: 0.0,
        }
    }
}

/// Marker for buildings that can provide jobs.
#[derive(Component)]
pub struct Workplace {
    pub job_capacity: usize,
    pub jobs_filled: usize,
}

/// Marker for buildings that house citizens.
#[derive(Component)]
pub struct Residence {
    pub capacity: usize,
    pub occupants: usize,
}

/// System to spawn citizens from residential buildings and assign jobs.
fn spawn_citizens(
    mut commands: Commands,
    config: Res<CitizenConfig>,
    buildings: Query<(Entity, &Building, &Transform)>,
    mut spawned: ResMut<CitizensSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Collect residential and job-providing buildings
    let mut residences: Vec<Entity> = Vec::new();
    let mut workplaces: Vec<Entity> = Vec::new();
    let mut total_jobs = 0usize;

    for (entity, building, _) in buildings.iter() {
        match building.building_type {
            BuildingArchetype::Residential => {
                let capacity = (config.citizens_per_residential * rng.gen_range(0.5..1.5)) as usize;
                commands.entity(entity).insert(Residence {
                    capacity: capacity.max(1),
                    occupants: 0,
                });
                residences.push(entity);
            }
            BuildingArchetype::Commercial => {
                let jobs = (config.jobs_per_commercial * rng.gen_range(0.5..1.5)) as usize;
                commands.entity(entity).insert(Workplace {
                    job_capacity: jobs.max(1),
                    jobs_filled: 0,
                });
                workplaces.push(entity);
                total_jobs += jobs;
            }
            BuildingArchetype::Industrial => {
                let jobs = (config.jobs_per_industrial * rng.gen_range(0.5..1.5)) as usize;
                commands.entity(entity).insert(Workplace {
                    job_capacity: jobs.max(1),
                    jobs_filled: 0,
                });
                workplaces.push(entity);
                total_jobs += jobs;
            }
            _ => {}
        }
    }

    if residences.is_empty() {
        info!("No residential buildings found for citizen spawning");
        return;
    }

    // Spawn citizens
    let mut citizen_count = 0usize;
    let mut employed_count = 0usize;
    let mut workplace_idx = 0usize;

    for &residence in residences.iter() {
        if citizen_count >= config.max_citizens {
            break;
        }

        let num_citizens = rng.gen_range(1..=6).min(config.max_citizens - citizen_count);

        for _ in 0..num_citizens {
            // Randomly assign a job (if available and within employment target)
            let should_be_employed = rng.gen::<f32>() < config.target_employment_rate;
            let work = if should_be_employed && !workplaces.is_empty() {
                let wp = workplaces[workplace_idx % workplaces.len()];
                workplace_idx += 1;
                employed_count += 1;
                Some(wp)
            } else {
                None
            };

            // Randomize schedule slightly
            let schedule = DailySchedule {
                wake_time: 6.0 + rng.gen_range(0.0..2.0),
                work_start: 8.0 + rng.gen_range(0.0..2.0),
                work_end: 16.0 + rng.gen_range(0.0..3.0),
                sleep_time: 22.0 + rng.gen_range(0.0..2.0),
            };

            // Randomize starting needs
            let needs = Needs {
                hunger: rng.gen_range(0.6..1.0),
                rest: rng.gen_range(0.6..1.0),
                income: if work.is_some() { rng.gen_range(0.4..0.7) } else { rng.gen_range(0.2..0.4) },
                happiness: rng.gen_range(0.5..0.8),
            };

            // Randomize age (working age)
            let age = rng.gen_range(18..65);

            commands.spawn((
                Citizen {
                    home: residence,
                    work,
                    needs,
                    state: CitizenState::AtHome,
                    age,
                    state_time: 0.0,
                },
                schedule,
            ));

            citizen_count += 1;
        }
    }

    info!(
        "Spawned {} citizens ({} employed, {} unemployed) across {} residences with {} jobs available",
        citizen_count,
        employed_count,
        citizen_count - employed_count,
        residences.len(),
        total_jobs
    );
}

/// Update citizen state based on time of day and schedule.
fn update_citizen_state(
    time_of_day: Option<Res<TimeOfDay>>,
    time: Res<Time>,
    mut citizens: Query<(&mut Citizen, &DailySchedule)>,
) {
    let Some(tod) = time_of_day else { return };
    let hour = tod.hour();
    let dt_hours = time.delta_secs() / 3600.0; // Convert to hours

    for (mut citizen, schedule) in citizens.iter_mut() {
        citizen.state_time += dt_hours;

        let new_state = determine_state(hour, &citizen, schedule);

        if new_state != citizen.state {
            citizen.state = new_state;
            citizen.state_time = 0.0;
        }
    }
}

/// Determine what state a citizen should be in based on time.
fn determine_state(hour: f32, citizen: &Citizen, schedule: &DailySchedule) -> CitizenState {
    let has_job = citizen.work.is_some();

    // Sleeping time
    if hour >= schedule.sleep_time || hour < schedule.wake_time {
        return CitizenState::Sleeping;
    }

    // Just woke up - at home briefly
    if hour >= schedule.wake_time && hour < schedule.wake_time + 0.5 {
        return CitizenState::AtHome;
    }

    // Commute to work
    if has_job && hour >= schedule.wake_time + 0.5 && hour < schedule.work_start {
        return CitizenState::Commuting;
    }

    // At work
    if has_job && hour >= schedule.work_start && hour < schedule.work_end {
        return CitizenState::AtWork;
    }

    // Commute home
    if has_job && hour >= schedule.work_end && hour < schedule.work_end + 0.5 {
        return CitizenState::Commuting;
    }

    // Evening activities
    if hour >= schedule.work_end + 0.5 && hour < schedule.sleep_time {
        // Unemployed or after work
        if citizen.needs.hunger < 0.5 {
            return CitizenState::Shopping; // Go shopping/eating
        }
        if citizen.needs.happiness < 0.5 {
            return CitizenState::Leisure;
        }
        return CitizenState::AtHome;
    }

    // Default: at home
    CitizenState::AtHome
}

/// Update citizen needs over time.
fn update_citizen_needs(
    config: Res<CitizenConfig>,
    time: Res<Time>,
    mut tick_events: EventReader<SimulationTick>,
    mut citizens: Query<&mut Citizen>,
) {
    // Only update on simulation ticks
    let tick_count = tick_events.read().count();
    if tick_count == 0 {
        return;
    }

    let dt_hours = time.delta_secs() / 3600.0 * tick_count as f32;

    for mut citizen in citizens.iter_mut() {
        // Decay needs over time
        citizen.needs.hunger = (citizen.needs.hunger - config.hunger_decay_rate * dt_hours).max(0.0);
        citizen.needs.rest = (citizen.needs.rest - config.rest_decay_rate * dt_hours).max(0.0);

        // State-based need changes
        match citizen.state {
            CitizenState::Sleeping => {
                // Restore rest while sleeping
                citizen.needs.rest = (citizen.needs.rest + 0.1 * dt_hours).min(1.0);
            }
            CitizenState::AtHome => {
                // Eat at home
                citizen.needs.hunger = (citizen.needs.hunger + 0.08 * dt_hours).min(1.0);
                // Slight rest recovery
                citizen.needs.rest = (citizen.needs.rest + 0.02 * dt_hours).min(1.0);
            }
            CitizenState::AtWork => {
                // Earn income while working
                citizen.needs.income = (citizen.needs.income + 0.05 * dt_hours).min(1.0);
                // Work is tiring
                citizen.needs.rest = (citizen.needs.rest - 0.02 * dt_hours).max(0.0);
            }
            CitizenState::Shopping => {
                // Spend money, get food
                citizen.needs.hunger = (citizen.needs.hunger + 0.15 * dt_hours).min(1.0);
                citizen.needs.income = (citizen.needs.income - 0.02 * dt_hours).max(0.0);
            }
            CitizenState::Leisure => {
                // Happiness boost
                citizen.needs.happiness = (citizen.needs.happiness + 0.1 * dt_hours).min(1.0);
                // Spend money on entertainment
                citizen.needs.income = (citizen.needs.income - 0.01 * dt_hours).max(0.0);
            }
            CitizenState::Commuting => {
                // Commuting is tiring
                citizen.needs.rest = (citizen.needs.rest - 0.01 * dt_hours).max(0.0);
                citizen.needs.happiness = (citizen.needs.happiness - 0.01 * dt_hours).max(0.0);
            }
        }

        // Calculate overall happiness from needs
        let base_happiness = (citizen.needs.hunger + citizen.needs.rest + citizen.needs.income) / 3.0;
        // Slowly blend toward base happiness
        citizen.needs.happiness = citizen.needs.happiness * 0.95 + base_happiness * 0.05;

        // Unemployed are less happy
        if citizen.work.is_none() {
            citizen.needs.happiness = (citizen.needs.happiness - config.happiness_decay_rate * dt_hours).max(0.0);
        }
    }
}

/// Update citizen statistics.
fn update_citizen_stats(
    citizens: Query<&Citizen>,
    mut stats: ResMut<CitizenStats>,
) {
    let mut total = 0;
    let mut employed = 0;
    let mut at_home = 0;
    let mut commuting = 0;
    let mut at_work = 0;
    let mut shopping = 0;
    let mut at_leisure = 0;
    let mut total_happiness = 0.0;
    let mut total_hunger = 0.0;
    let mut total_rest = 0.0;

    for citizen in citizens.iter() {
        total += 1;

        if citizen.work.is_some() {
            employed += 1;
        }

        match citizen.state {
            CitizenState::AtHome | CitizenState::Sleeping => at_home += 1,
            CitizenState::Commuting => commuting += 1,
            CitizenState::AtWork => at_work += 1,
            CitizenState::Shopping => shopping += 1,
            CitizenState::Leisure => at_leisure += 1,
        }

        total_happiness += citizen.needs.happiness;
        total_hunger += citizen.needs.hunger;
        total_rest += citizen.needs.rest;
    }

    stats.total_citizens = total;
    stats.employed = employed;
    stats.unemployed = total.saturating_sub(employed);
    stats.at_home = at_home;
    stats.commuting = commuting;
    stats.at_work = at_work;
    stats.shopping = shopping;
    stats.at_leisure = at_leisure;

    if total > 0 {
        stats.average_happiness = total_happiness / total as f32;
        stats.average_hunger = total_hunger / total as f32;
        stats.average_rest = total_rest / total as f32;
    }
}

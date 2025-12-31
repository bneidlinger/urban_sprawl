//! Citizen agent system.

#![allow(dead_code)]

use bevy::prelude::*;

/// Citizen needs (Maslow-lite).
#[derive(Clone, Copy, Debug, Default)]
pub struct Needs {
    pub hunger: f32,
    pub rest: f32,
    pub income: f32,
    pub happiness: f32,
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
}

/// Citizen component.
#[derive(Component)]
pub struct Citizen {
    pub home: Entity,
    pub work: Option<Entity>,
    pub needs: Needs,
    pub state: CitizenState,
    pub age: u32,
}

impl Default for Citizen {
    fn default() -> Self {
        Self {
            home: Entity::PLACEHOLDER,
            work: None,
            needs: Needs::default(),
            state: CitizenState::AtHome,
            age: 30,
        }
    }
}

/// Schedule for citizen activities (24-hour cycle).
#[derive(Clone, Debug)]
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

//! Clustered shading system for efficient city lighting.
//!
//! This module manages thousands of point lights for street lamps, traffic lights,
//! and window lights using Bevy's built-in clustered forward rendering.
//!
//! Bevy automatically subdivides the view frustum into a 3D grid of clusters
//! and assigns lights to overlapping clusters for efficient per-fragment lighting.

use bevy::prelude::*;

pub mod cluster_config;
pub mod light_buffer;

pub use cluster_config::ClusterConfig;
pub use light_buffer::{CityLight, CityLightBuffer, LightType};

use crate::render::day_night::TimeOfDay;

pub struct ClusteredShadingPlugin;

impl Plugin for ClusteredShadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClusterConfig>()
            .init_resource::<CityLightBuffer>()
            .init_resource::<LightingStats>()
            .add_systems(
                Update,
                (
                    update_light_intensities,
                    update_lighting_stats,
                )
                    .chain(),
            );
    }
}

/// Statistics about active lights in the scene.
#[derive(Resource, Default)]
pub struct LightingStats {
    pub total_lights: usize,
    pub active_lights: usize,
    pub street_lamps: usize,
    pub traffic_lights: usize,
    pub window_lights: usize,
}

/// Component marking an entity as a dynamic city light.
/// The intensity will be automatically adjusted based on time of day.
#[derive(Component)]
pub struct DynamicCityLight {
    pub light_type: LightType,
    /// Base intensity when fully on
    pub base_intensity: f32,
    /// Current intensity multiplier (0.0-1.0)
    pub current_factor: f32,
}

impl DynamicCityLight {
    pub fn street_lamp(intensity: f32) -> Self {
        Self {
            light_type: LightType::StreetLamp,
            base_intensity: intensity,
            current_factor: 0.0,
        }
    }

    pub fn traffic_light(intensity: f32) -> Self {
        Self {
            light_type: LightType::TrafficLight,
            base_intensity: intensity,
            current_factor: 1.0, // Traffic lights always on
        }
    }

    pub fn window_light(intensity: f32) -> Self {
        Self {
            light_type: LightType::Window,
            base_intensity: intensity,
            current_factor: 0.0,
        }
    }

    pub fn entrance_light(intensity: f32) -> Self {
        Self {
            light_type: LightType::Entrance,
            base_intensity: intensity,
            current_factor: 0.0,
        }
    }
}

/// Update light intensities based on time of day.
fn update_light_intensities(
    tod: Res<TimeOfDay>,
    mut light_query: Query<(&mut PointLight, &mut DynamicCityLight)>,
) {
    let hour = tod.hour();

    // Calculate night factor for different light types
    let street_lamp_factor = calculate_street_lamp_factor(hour);
    let window_factor = calculate_window_factor(hour);
    let entrance_factor = calculate_entrance_factor(hour);

    for (mut point_light, mut city_light) in light_query.iter_mut() {
        let factor = match city_light.light_type {
            LightType::StreetLamp => street_lamp_factor,
            LightType::TrafficLight => 1.0, // Always on (reduced during day)
            LightType::Window => window_factor,
            LightType::Vehicle => 1.0, // Handled separately
            LightType::Entrance => entrance_factor,
        };

        city_light.current_factor = factor;
        point_light.intensity = city_light.base_intensity * factor;
    }
}

/// Calculate street lamp intensity factor based on hour.
fn calculate_street_lamp_factor(hour: f32) -> f32 {
    if hour >= 6.0 && hour <= 7.0 {
        // Dawn - lamps turning off
        1.0 - (hour - 6.0)
    } else if hour >= 18.0 && hour <= 19.0 {
        // Dusk - lamps turning on
        hour - 18.0
    } else if hour > 7.0 && hour < 18.0 {
        // Day - lamps off (minimal glow)
        0.05
    } else {
        // Night - lamps fully on
        1.0
    }
}

/// Calculate window light intensity factor based on hour.
fn calculate_window_factor(hour: f32) -> f32 {
    if hour >= 6.0 && hour <= 8.0 {
        // Morning - some windows turning off
        1.0 - (hour - 6.0) / 2.0
    } else if hour >= 17.0 && hour <= 19.0 {
        // Evening - windows turning on
        (hour - 17.0) / 2.0
    } else if hour > 8.0 && hour < 17.0 {
        // Day - minimal windows lit
        0.1
    } else {
        // Night - windows on
        0.7 // Not all windows are lit
    }
}

/// Calculate entrance light intensity factor based on hour.
/// Entrance lights turn on slightly earlier than street lamps (dusk) and stay on later.
fn calculate_entrance_factor(hour: f32) -> f32 {
    if hour >= 6.5 && hour <= 7.5 {
        // Morning - entrance lights turning off
        1.0 - (hour - 6.5)
    } else if hour >= 17.0 && hour <= 18.0 {
        // Early evening - entrance lights turning on (before street lamps)
        hour - 17.0
    } else if hour > 7.5 && hour < 17.0 {
        // Day - entrance lights mostly off
        0.08
    } else {
        // Night - entrance lights fully on
        1.0
    }
}

/// Update lighting statistics.
fn update_lighting_stats(
    light_query: Query<&DynamicCityLight>,
    mut stats: ResMut<LightingStats>,
) {
    let mut total = 0;
    let mut active = 0;
    let mut street = 0;
    let mut traffic = 0;
    let mut window = 0;

    for light in light_query.iter() {
        total += 1;
        if light.current_factor > 0.1 {
            active += 1;
        }
        match light.light_type {
            LightType::StreetLamp => street += 1,
            LightType::TrafficLight => traffic += 1,
            LightType::Window => window += 1,
            LightType::Vehicle | LightType::Entrance => {},
        }
    }

    stats.total_lights = total;
    stats.active_lights = active;
    stats.street_lamps = street;
    stats.traffic_lights = traffic;
    stats.window_lights = window;
}

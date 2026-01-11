//! Ambient audio system for city atmosphere.
//!
//! Plays layered ambient sounds based on time of day, weather, and
//! camera position. Uses Bevy's spatial audio for immersive experience.

use bevy::prelude::*;

use crate::render::day_night::TimeOfDay;
use crate::render::weather::{Weather, WeatherState};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientAudioConfig>()
            .init_resource::<AmbientAudioState>()
            .add_systems(Startup, setup_ambient_audio)
            .add_systems(
                Update,
                (
                    update_ambient_layers,
                    update_traffic_audio,
                    update_weather_audio,
                ),
            );
    }
}

/// Configuration for ambient audio.
#[derive(Resource)]
pub struct AmbientAudioConfig {
    /// Master volume (0.0 to 1.0).
    pub master_volume: f32,
    /// City ambience volume.
    pub city_volume: f32,
    /// Traffic volume.
    pub traffic_volume: f32,
    /// Weather volume (rain, thunder).
    pub weather_volume: f32,
    /// Whether audio is enabled.
    pub enabled: bool,
}

impl Default for AmbientAudioConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.7,
            city_volume: 0.5,
            traffic_volume: 0.4,
            weather_volume: 0.6,
            enabled: true,
        }
    }
}

/// Current state of ambient audio playback.
#[derive(Resource, Default)]
pub struct AmbientAudioState {
    /// Currently playing city layer.
    pub city_layer: Option<Entity>,
    /// Currently playing traffic layer.
    pub traffic_layer: Option<Entity>,
    /// Currently playing weather layer.
    pub weather_layer: Option<Entity>,
    /// Target volume for city (for smooth transitions).
    pub city_target_volume: f32,
    /// Target volume for traffic.
    pub traffic_target_volume: f32,
    /// Target volume for weather.
    pub weather_target_volume: f32,
    /// Current city volume (interpolating toward target).
    pub city_current_volume: f32,
    /// Current traffic volume.
    pub traffic_current_volume: f32,
    /// Current weather volume.
    pub weather_current_volume: f32,
}

/// Marker for ambient audio source entities.
#[derive(Component)]
pub struct AmbientAudioSource {
    pub layer: AmbientLayer,
}

/// Different layers of ambient audio.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AmbientLayer {
    /// City background hum (HVAC, distant voices, etc.).
    City,
    /// Traffic sounds.
    Traffic,
    /// Weather sounds (rain, wind, thunder).
    Weather,
}

/// Set up ambient audio sources.
fn setup_ambient_audio(mut state: ResMut<AmbientAudioState>) {
    // Initialize target volumes
    state.city_target_volume = 0.5;
    state.traffic_target_volume = 0.3;
    state.weather_target_volume = 0.0;

    // Note: Actual audio playback would require audio assets to be loaded.
    // For now, we set up the state machine. Audio files would be:
    // - assets/audio/city_day.ogg
    // - assets/audio/city_night.ogg
    // - assets/audio/traffic_light.ogg
    // - assets/audio/traffic_heavy.ogg
    // - assets/audio/rain_light.ogg
    // - assets/audio/rain_heavy.ogg
    // - assets/audio/thunder.ogg
    // - assets/audio/wind.ogg

    info!("Ambient audio system initialized (placeholder - needs audio assets)");
}

/// Update ambient audio volumes based on time of day.
fn update_ambient_layers(
    tod: Option<Res<TimeOfDay>>,
    config: Res<AmbientAudioConfig>,
    mut state: ResMut<AmbientAudioState>,
    time: Res<Time>,
) {
    if !config.enabled {
        state.city_target_volume = 0.0;
        state.traffic_target_volume = 0.0;
        return;
    }

    let hour = tod.as_ref().map(|t| t.hour()).unwrap_or(12.0);

    // Calculate time-of-day factors
    let (city_factor, traffic_factor) = if hour >= 6.0 && hour <= 9.0 {
        // Morning rush hour - high traffic
        (0.6, 0.9)
    } else if hour >= 9.0 && hour <= 17.0 {
        // Daytime - moderate activity
        (0.5, 0.6)
    } else if hour >= 17.0 && hour <= 20.0 {
        // Evening rush hour - high traffic
        (0.7, 0.85)
    } else if hour >= 20.0 && hour <= 23.0 {
        // Evening - moderate city, low traffic
        (0.6, 0.4)
    } else {
        // Night - low activity
        (0.3, 0.2)
    };

    // Set target volumes
    state.city_target_volume = city_factor * config.city_volume * config.master_volume;
    state.traffic_target_volume = traffic_factor * config.traffic_volume * config.master_volume;

    // Smoothly interpolate current volumes toward targets
    let lerp_speed = 0.5 * time.delta_secs();
    state.city_current_volume +=
        (state.city_target_volume - state.city_current_volume) * lerp_speed;
    state.traffic_current_volume +=
        (state.traffic_target_volume - state.traffic_current_volume) * lerp_speed;
}

/// Update traffic audio based on traffic density.
fn update_traffic_audio(
    config: Res<AmbientAudioConfig>,
    mut state: ResMut<AmbientAudioState>,
) {
    if !config.enabled {
        return;
    }

    // TODO: Sample traffic density from TrafficCaStats
    // For now, use the base traffic volume from time-of-day calculation
}

/// Update weather audio based on current weather state.
fn update_weather_audio(
    weather: Option<Res<WeatherState>>,
    config: Res<AmbientAudioConfig>,
    mut state: ResMut<AmbientAudioState>,
    time: Res<Time>,
) {
    if !config.enabled {
        state.weather_target_volume = 0.0;
        return;
    }

    let current_weather = weather
        .as_ref()
        .map(|w| w.current)
        .unwrap_or(Weather::Clear);

    // Set target volume based on weather
    let weather_factor = match current_weather {
        Weather::Clear => 0.0,
        Weather::Foggy => 0.1, // Light wind
        Weather::Rainy => 0.7, // Rain sounds
        Weather::Stormy => 1.0, // Heavy rain + thunder
    };

    state.weather_target_volume = weather_factor * config.weather_volume * config.master_volume;

    // Smooth interpolation
    let lerp_speed = 0.3 * time.delta_secs();
    state.weather_current_volume +=
        (state.weather_target_volume - state.weather_current_volume) * lerp_speed;
}

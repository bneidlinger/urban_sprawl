//! Weather system with dynamic fog, rain, and wet surfaces.
//!
//! Provides weather states (Clear, Foggy, Rainy, Stormy) that modulate
//! the base day/night fog and add visual effects like rain and wet roads.

use bevy::{
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use rand::prelude::*;

use super::cloud_shadows::CloudShadowConfig;
use super::day_night::{fog_profile, lerp_color, lerp_scalar, DayNightConfig, TimeOfDay};

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<RainMaterial>::default())
            .add_plugins(MaterialPlugin::<WetSurfaceMaterial>::default())
            .init_resource::<WeatherConfig>()
            .init_resource::<WeatherState>()
            .add_systems(Startup, (spawn_rain_plane, spawn_wet_surface_overlay))
            .add_systems(
                Update,
                (
                    handle_weather_input,
                    advance_weather_cycle,
                    update_weather_transition,
                )
                    .chain(),
            )
            // Run fog modification after day_night systems complete
            .add_systems(
                PostUpdate,
                (
                    apply_weather_fog,
                    update_rain_effect,
                    update_wet_surfaces,
                    sync_clouds_to_weather,
                ),
            );
    }
}

/// Weather state enum (no Box<dyn Trait> per CLAUDE.md constraints).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Weather {
    #[default]
    Clear,
    Foggy,
    Rainy,
    Stormy,
}

impl Weather {
    /// Cycle to the next weather state.
    pub fn next(self) -> Self {
        match self {
            Weather::Clear => Weather::Foggy,
            Weather::Foggy => Weather::Rainy,
            Weather::Rainy => Weather::Stormy,
            Weather::Stormy => Weather::Clear,
        }
    }

    /// Get fog density multiplier for this weather.
    pub fn fog_density_multiplier(self) -> f32 {
        match self {
            Weather::Clear => 1.0,
            Weather::Foggy => 2.5,
            Weather::Rainy => 1.5,
            Weather::Stormy => 3.0,
        }
    }

    /// Get fog color tint for this weather.
    pub fn fog_tint(self) -> Color {
        match self {
            Weather::Clear => Color::srgba(1.0, 1.0, 1.0, 0.0), // No tint
            Weather::Foggy => Color::srgba(0.85, 0.85, 0.88, 0.6),
            Weather::Rainy => Color::srgba(0.55, 0.60, 0.70, 0.4),
            Weather::Stormy => Color::srgba(0.35, 0.38, 0.45, 0.7),
        }
    }

    /// Get rain intensity for this weather (0.0 = no rain).
    pub fn rain_intensity(self) -> f32 {
        match self {
            Weather::Clear | Weather::Foggy => 0.0,
            Weather::Rainy => 0.6,
            Weather::Stormy => 1.0,
        }
    }

    /// Get wetness level for this weather.
    pub fn wetness(self) -> f32 {
        match self {
            Weather::Clear => 0.0,
            Weather::Foggy => 0.1,
            Weather::Rainy => 0.7,
            Weather::Stormy => 1.0,
        }
    }

    /// Get cloud coverage modifier for this weather.
    pub fn cloud_coverage(self) -> f32 {
        match self {
            Weather::Clear => 0.0,
            Weather::Foggy => 0.3,
            Weather::Rainy => 0.6,
            Weather::Stormy => 0.9,
        }
    }
}

/// Current weather state with transition tracking.
#[derive(Resource)]
pub struct WeatherState {
    /// Current active weather.
    pub current: Weather,
    /// Target weather (for transitions).
    pub target: Weather,
    /// Transition progress (0.0 to 1.0).
    pub transition: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current: Weather::Clear,
            target: Weather::Clear,
            transition: 1.0,
        }
    }
}

impl WeatherState {
    /// Get interpolated fog multiplier during transition.
    pub fn fog_multiplier(&self) -> f32 {
        lerp_scalar(
            self.current.fog_density_multiplier(),
            self.target.fog_density_multiplier(),
            self.transition,
        )
    }

    /// Get interpolated fog tint during transition.
    pub fn fog_tint(&self) -> Color {
        lerp_color(self.current.fog_tint(), self.target.fog_tint(), self.transition)
    }

    /// Get interpolated rain intensity during transition.
    pub fn rain_intensity(&self) -> f32 {
        lerp_scalar(
            self.current.rain_intensity(),
            self.target.rain_intensity(),
            self.transition,
        )
    }

    /// Get interpolated wetness during transition.
    pub fn wetness(&self) -> f32 {
        lerp_scalar(self.current.wetness(), self.target.wetness(), self.transition)
    }

    /// Get interpolated cloud coverage during transition.
    pub fn cloud_coverage(&self) -> f32 {
        lerp_scalar(
            self.current.cloud_coverage(),
            self.target.cloud_coverage(),
            self.transition,
        )
    }

    /// Check if currently transitioning.
    pub fn is_transitioning(&self) -> bool {
        self.current != self.target
    }
}

/// Configuration for weather system behavior.
#[derive(Resource)]
pub struct WeatherConfig {
    /// Duration of weather transitions in seconds.
    pub transition_duration: f32,
    /// Whether automatic weather cycling is enabled.
    pub auto_cycle_enabled: bool,
    /// Minimum time before weather changes (seconds).
    pub min_cycle_time: f32,
    /// Maximum time before weather changes (seconds).
    pub max_cycle_time: f32,
    /// Time until next automatic weather change.
    pub next_cycle_timer: f32,
    /// Rain fall speed multiplier.
    pub rain_speed: f32,
    /// Rain wind angle in radians.
    pub rain_angle: f32,
    /// Size of the rain/wet surface planes.
    pub plane_size: f32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            transition_duration: 10.0,
            auto_cycle_enabled: true,
            min_cycle_time: 600.0,  // 10 minutes
            max_cycle_time: 1800.0, // 30 minutes
            next_cycle_timer: 900.0,
            rain_speed: 8.0,
            rain_angle: 0.1,
            plane_size: 800.0,
        }
    }
}

// ============================================================================
// Rain Material
// ============================================================================

/// Custom material for procedural rain effect.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct RainMaterial {
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub intensity: f32,
    #[uniform(0)]
    pub speed: f32,
    #[uniform(0)]
    pub angle: f32,
    #[uniform(0)]
    pub opacity: f32,
    #[uniform(0)]
    pub _padding: Vec3,
}

impl Material for RainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/rain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

impl Default for RainMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            intensity: 0.0,
            speed: 8.0,
            angle: 0.1,
            opacity: 0.0,
            _padding: Vec3::ZERO,
        }
    }
}

/// Marker for the rain plane entity.
#[derive(Component)]
pub struct RainPlane;

// ============================================================================
// Wet Surface Material
// ============================================================================

/// Custom material for wet road/surface overlay.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct WetSurfaceMaterial {
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub wetness: f32,
    #[uniform(0)]
    pub puddle_coverage: f32,
    #[uniform(0)]
    pub reflection_strength: f32,
    #[uniform(0)]
    pub sky_color: LinearRgba,
}

impl Material for WetSurfaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/wet_surface.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

impl Default for WetSurfaceMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            wetness: 0.0,
            puddle_coverage: 0.0,
            reflection_strength: 0.0,
            sky_color: LinearRgba::new(0.5, 0.6, 0.8, 1.0),
        }
    }
}

/// Marker for the wet surface overlay entity.
#[derive(Component)]
pub struct WetSurfaceOverlay;

// ============================================================================
// Startup Systems
// ============================================================================

fn spawn_rain_plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RainMaterial>>,
    config: Res<WeatherConfig>,
) {
    let plane_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(config.plane_size, config.plane_size),
    );

    let material = materials.add(RainMaterial::default());

    // Position rain plane high above the city, tilted slightly toward camera
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 80.0, 0.0).with_rotation(Quat::from_rotation_x(-0.15)),
        RainPlane,
    ));

    info!("Rain plane spawned at height 80");
}

fn spawn_wet_surface_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WetSurfaceMaterial>>,
    config: Res<WeatherConfig>,
) {
    let plane_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(config.plane_size, config.plane_size),
    );

    let material = materials.add(WetSurfaceMaterial::default());

    // Position just above ground (like cloud shadows)
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.6, 0.0),
        WetSurfaceOverlay,
    ));

    info!("Wet surface overlay spawned at height 0.6");
}

// ============================================================================
// Input System
// ============================================================================

fn handle_weather_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut weather: ResMut<WeatherState>,
    mut config: ResMut<WeatherConfig>,
) {
    // Cycle weather with F key (but not when Shift is held)
    if keyboard.just_pressed(KeyCode::KeyF) && !keyboard.pressed(KeyCode::ShiftLeft) {
        let new_target = weather.current.next();
        if new_target != weather.target {
            weather.target = new_target;
            weather.transition = 0.0;
            info!("Weather cycling to {:?}", new_target);
        }
    }

    // Toggle auto-cycling with Shift+F
    if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyF) {
        config.auto_cycle_enabled = !config.auto_cycle_enabled;
        info!("Weather auto-cycle: {}", config.auto_cycle_enabled);
    }

    // Direct weather selection with F5-F8
    let direct_weather = if keyboard.just_pressed(KeyCode::F5) {
        Some(Weather::Clear)
    } else if keyboard.just_pressed(KeyCode::F6) {
        Some(Weather::Foggy)
    } else if keyboard.just_pressed(KeyCode::F7) {
        Some(Weather::Rainy)
    } else if keyboard.just_pressed(KeyCode::F8) {
        Some(Weather::Stormy)
    } else {
        None
    };

    if let Some(new_weather) = direct_weather {
        if new_weather != weather.current || weather.is_transitioning() {
            weather.target = new_weather;
            weather.transition = 0.0;
            info!("Weather set to {:?}", new_weather);
        }
    }
}

// ============================================================================
// Weather Cycle System
// ============================================================================

fn advance_weather_cycle(
    time: Res<Time>,
    mut weather: ResMut<WeatherState>,
    mut config: ResMut<WeatherConfig>,
) {
    if !config.auto_cycle_enabled {
        return;
    }

    // Only count down when not transitioning
    if !weather.is_transitioning() {
        config.next_cycle_timer -= time.delta_secs();

        if config.next_cycle_timer <= 0.0 {
            // Pick a random different weather
            let mut rng = thread_rng();
            let options: Vec<Weather> = [Weather::Clear, Weather::Foggy, Weather::Rainy, Weather::Stormy]
                .into_iter()
                .filter(|&w| w != weather.current)
                .collect();

            if let Some(&new_weather) = options.get(rng.gen_range(0..options.len())) {
                weather.target = new_weather;
                weather.transition = 0.0;

                // Set next cycle timer
                config.next_cycle_timer = rng.gen_range(config.min_cycle_time..config.max_cycle_time);

                info!(
                    "Auto-cycling weather to {:?} (next in {:.0}s)",
                    new_weather, config.next_cycle_timer
                );
            }
        }
    }
}

// ============================================================================
// Transition System
// ============================================================================

fn update_weather_transition(time: Res<Time>, config: Res<WeatherConfig>, mut weather: ResMut<WeatherState>) {
    if !weather.is_transitioning() {
        return;
    }

    weather.transition += time.delta_secs() / config.transition_duration;

    if weather.transition >= 1.0 {
        weather.current = weather.target;
        weather.transition = 1.0;
        info!("Weather transitioned to {:?}", weather.current);
    }
}

// ============================================================================
// Fog Modification System
// ============================================================================

fn apply_weather_fog(
    weather: Res<WeatherState>,
    tod: Res<TimeOfDay>,
    day_night_config: Res<DayNightConfig>,
    mut fog_query: Query<&mut DistanceFog, With<Camera3d>>,
) {
    let hour = tod.hour();
    let (base_color, base_density) = fog_profile(hour, &day_night_config);

    // Get weather modifiers
    let density_mult = weather.fog_multiplier();
    let weather_tint = weather.fog_tint();

    for mut fog in fog_query.iter_mut() {
        // Blend weather tint with base fog color
        let tint_alpha = weather_tint.alpha();
        let final_color = if tint_alpha > 0.01 {
            lerp_color(base_color, weather_tint, tint_alpha)
        } else {
            base_color
        };

        fog.color = final_color;
        fog.falloff = FogFalloff::Exponential {
            density: base_density * density_mult,
        };
    }
}

// ============================================================================
// Rain Effect System
// ============================================================================

fn update_rain_effect(
    time: Res<Time>,
    weather: Res<WeatherState>,
    config: Res<WeatherConfig>,
    mut materials: ResMut<Assets<RainMaterial>>,
    query: Query<&MeshMaterial3d<RainMaterial>, With<RainPlane>>,
) {
    let intensity = weather.rain_intensity();

    // Calculate wind angle variation during storms
    let angle = if weather.current == Weather::Stormy || weather.target == Weather::Stormy {
        config.rain_angle + (time.elapsed_secs() * 0.2).sin() * 0.15
    } else {
        config.rain_angle
    };

    for handle in query.iter() {
        if let Some(material) = materials.get_mut(handle) {
            material.time = time.elapsed_secs();
            material.intensity = intensity;
            material.speed = config.rain_speed;
            material.angle = angle;
            material.opacity = intensity * 0.7;
        }
    }
}

// ============================================================================
// Wet Surface System
// ============================================================================

fn update_wet_surfaces(
    time: Res<Time>,
    weather: Res<WeatherState>,
    clear_color: Res<ClearColor>,
    mut materials: ResMut<Assets<WetSurfaceMaterial>>,
    query: Query<&MeshMaterial3d<WetSurfaceMaterial>, With<WetSurfaceOverlay>>,
) {
    let wetness = weather.wetness();

    for handle in query.iter() {
        if let Some(material) = materials.get_mut(handle) {
            material.time = time.elapsed_secs();
            material.wetness = wetness;
            material.puddle_coverage = wetness * 0.5;
            material.reflection_strength = wetness * 0.4;
            material.sky_color = clear_color.0.to_linear();
        }
    }
}

// ============================================================================
// Cloud Integration System
// ============================================================================

fn sync_clouds_to_weather(weather: Res<WeatherState>, mut cloud_config: Option<ResMut<CloudShadowConfig>>) {
    if let Some(ref mut config) = cloud_config {
        let weather_coverage = weather.cloud_coverage();

        // Base coverage from CloudShadowConfig default is 0.45
        let base_coverage = 0.45;
        config.coverage = base_coverage + weather_coverage * 0.4;
        config.max_opacity = 0.35 + weather_coverage * 0.3;
    }
}

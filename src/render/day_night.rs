//! Day/night cycle with sun animation and atmospheric changes.

use bevy::{
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeOfDay>()
            .init_resource::<DayNightConfig>()
            // Shadow map resolution (2048 is good balance of quality/performance)
            .insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_systems(Startup, setup_lighting)
            .add_systems(Update, (
                advance_time,
                update_sun_position,
                update_ambient_light,
                update_sky_color,
            ).chain());
    }
}

/// Current time of day (0.0 = midnight, 0.5 = noon, 1.0 = midnight)
#[derive(Resource)]
pub struct TimeOfDay {
    /// Normalized time (0.0 to 1.0)
    pub time: f32,
    /// Speed multiplier (1.0 = 24 minutes per full cycle)
    pub speed: f32,
    /// Whether time is paused
    pub paused: bool,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            time: 0.35, // Start at morning (8:24 AM)
            speed: 0.5,  // Half speed for nice viewing
            paused: false,
        }
    }
}

impl TimeOfDay {
    /// Get hour of day (0-24)
    pub fn hour(&self) -> f32 {
        self.time * 24.0
    }

    /// Check if it's nighttime (before 6 AM or after 8 PM)
    pub fn is_night(&self) -> bool {
        let hour = self.hour();
        hour < 6.0 || hour > 20.0
    }

    /// Get interpolation factor for dusk/dawn transitions
    pub fn transition_factor(&self) -> f32 {
        let hour = self.hour();
        if hour >= 5.0 && hour <= 7.0 {
            // Dawn: 5 AM to 7 AM
            (hour - 5.0) / 2.0
        } else if hour >= 18.0 && hour <= 20.0 {
            // Dusk: 6 PM to 8 PM
            1.0 - (hour - 18.0) / 2.0
        } else if hour > 7.0 && hour < 18.0 {
            // Day
            1.0
        } else {
            // Night
            0.0
        }
    }
}

#[derive(Resource)]
pub struct DayNightConfig {
    // Sun settings
    pub sun_intensity_day: f32,
    pub sun_intensity_night: f32,

    // Ambient settings
    pub ambient_day: Color,
    pub ambient_night: Color,
    pub ambient_dawn: Color,
    pub ambient_dusk: Color,

    // Sky colors
    pub sky_day: Color,
    pub sky_dawn: Color,
    pub sky_dusk: Color,
    pub sky_night: Color,
}

impl Default for DayNightConfig {
    fn default() -> Self {
        Self {
            sun_intensity_day: 100000.0,
            sun_intensity_night: 500.0,

            ambient_day: Color::srgb(0.4, 0.45, 0.5),
            ambient_night: Color::srgb(0.02, 0.03, 0.08),
            ambient_dawn: Color::srgb(0.5, 0.35, 0.3),
            ambient_dusk: Color::srgb(0.6, 0.35, 0.25),

            sky_day: Color::srgb(0.5, 0.7, 0.9),
            sky_dawn: Color::srgb(0.9, 0.6, 0.4),
            sky_dusk: Color::srgb(0.9, 0.4, 0.3),
            sky_night: Color::srgb(0.02, 0.02, 0.05),
        }
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

fn setup_lighting(mut commands: Commands) {
    // Ambient light (updated by update_ambient_light system)
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Main directional light (sun) with cascaded shadow maps
    commands.spawn((
        DirectionalLight {
            illuminance: 100000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.3,
            shadow_normal_bias: 1.8,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
        // Cascade config for city-scale scene (~500 units)
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.1,
            maximum_distance: 400.0,
            first_cascade_far_bound: 50.0,
            overlap_proportion: 0.3,
        }
        .build(),
        Sun,
    ));

    // Moonlight (dimmer, blue-tinted)
    commands.spawn((
        DirectionalLight {
            illuminance: 500.0,
            color: Color::srgb(0.7, 0.8, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_3,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
        Moon,
    ));
}

fn advance_time(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Toggle pause with P
    if keyboard.just_pressed(KeyCode::KeyP) {
        tod.paused = !tod.paused;
    }

    // Speed controls with [ and ]
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        tod.speed = (tod.speed * 0.5).max(0.1);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        tod.speed = (tod.speed * 2.0).min(10.0);
    }

    // Jump to specific times with number keys
    if keyboard.just_pressed(KeyCode::Digit1) {
        tod.time = 0.25; // 6 AM (dawn)
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        tod.time = 0.5; // Noon
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        tod.time = 0.75; // 6 PM (dusk)
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        tod.time = 0.0; // Midnight
    }

    if !tod.paused {
        // One full cycle = 24 minutes at speed 1.0
        let cycle_duration = 24.0 * 60.0; // seconds
        tod.time += time.delta_secs() * tod.speed / cycle_duration;
        tod.time = tod.time.fract(); // Wrap around
    }
}

fn update_sun_position(
    tod: Res<TimeOfDay>,
    mut sun_query: Query<(&mut Transform, &mut DirectionalLight), (With<Sun>, Without<Moon>)>,
    mut moon_query: Query<(&mut Transform, &mut DirectionalLight), (With<Moon>, Without<Sun>)>,
    config: Res<DayNightConfig>,
) {
    // Sun angle: rises in east, sets in west
    // At time=0.25 (6AM): sun at horizon (east)
    // At time=0.5 (noon): sun at zenith
    // At time=0.75 (6PM): sun at horizon (west)

    let sun_angle = (tod.time - 0.25) * std::f32::consts::TAU;

    // Sun height (0 at horizon, 1 at zenith)
    let sun_height = (sun_angle).sin();

    // Sun azimuth
    let sun_azimuth = (sun_angle).cos();

    for (mut transform, mut light) in sun_query.iter_mut() {
        // Calculate sun direction
        let pitch = sun_height.asin().max(-0.1); // Keep slightly above horizon
        let yaw = sun_azimuth.atan2(1.0);

        *transform = Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            -pitch - 0.3, // Offset for better shadows
            0.0,
        ));

        // Adjust intensity based on height
        let day_factor = sun_height.max(0.0);
        light.illuminance = config.sun_intensity_night +
            (config.sun_intensity_day - config.sun_intensity_night) * day_factor;

        // Warm color at sunrise/sunset
        let transition = tod.transition_factor();
        if transition < 0.5 {
            // Dawn/dusk - warm orange
            light.color = Color::srgb(
                1.0,
                0.7 + transition * 0.3,
                0.5 + transition * 0.5,
            );
        } else {
            // Day - white
            light.color = Color::WHITE;
        }
    }

    // Moon - opposite position
    let moon_angle = sun_angle + std::f32::consts::PI;
    let moon_height = moon_angle.sin();

    for (mut transform, mut light) in moon_query.iter_mut() {
        let pitch = moon_height.asin().max(-0.1);
        let yaw = moon_angle.cos().atan2(1.0);

        *transform = Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            -pitch - 0.2,
            0.0,
        ));

        // Moon visible at night
        let night_factor = (-sun_height).max(0.0);
        light.illuminance = config.sun_intensity_night * night_factor * 2.0;
    }
}

fn update_ambient_light(
    tod: Res<TimeOfDay>,
    config: Res<DayNightConfig>,
    mut ambient: ResMut<AmbientLight>,
) {
    let hour = tod.hour();

    let color = if hour >= 5.0 && hour < 7.0 {
        // Dawn transition
        let t = (hour - 5.0) / 2.0;
        lerp_color(config.ambient_night, config.ambient_dawn, t)
    } else if hour >= 7.0 && hour < 8.0 {
        // Dawn to day
        let t = hour - 7.0;
        lerp_color(config.ambient_dawn, config.ambient_day, t)
    } else if hour >= 8.0 && hour < 17.0 {
        // Day
        config.ambient_day
    } else if hour >= 17.0 && hour < 18.0 {
        // Day to dusk
        let t = hour - 17.0;
        lerp_color(config.ambient_day, config.ambient_dusk, t)
    } else if hour >= 18.0 && hour < 20.0 {
        // Dusk transition
        let t = (hour - 18.0) / 2.0;
        lerp_color(config.ambient_dusk, config.ambient_night, t)
    } else {
        // Night
        config.ambient_night
    };

    ambient.color = color;
    ambient.brightness = if tod.is_night() { 50.0 } else { 300.0 };
}

fn update_sky_color(
    tod: Res<TimeOfDay>,
    config: Res<DayNightConfig>,
    mut clear_color: ResMut<ClearColor>,
) {
    let hour = tod.hour();

    let color = if hour >= 5.0 && hour < 7.0 {
        // Dawn
        let t = (hour - 5.0) / 2.0;
        lerp_color(config.sky_night, config.sky_dawn, t)
    } else if hour >= 7.0 && hour < 9.0 {
        // Dawn to day
        let t = (hour - 7.0) / 2.0;
        lerp_color(config.sky_dawn, config.sky_day, t)
    } else if hour >= 9.0 && hour < 17.0 {
        // Day
        config.sky_day
    } else if hour >= 17.0 && hour < 19.0 {
        // Day to dusk
        let t = (hour - 17.0) / 2.0;
        lerp_color(config.sky_day, config.sky_dusk, t)
    } else if hour >= 19.0 && hour < 21.0 {
        // Dusk to night
        let t = (hour - 19.0) / 2.0;
        lerp_color(config.sky_dusk, config.sky_night, t)
    } else {
        // Night
        config.sky_night
    };

    clear_color.0 = color;
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a_linear = a.to_linear();
    let b_linear = b.to_linear();

    Color::linear_rgb(
        a_linear.red + (b_linear.red - a_linear.red) * t,
        a_linear.green + (b_linear.green - a_linear.green) * t,
        a_linear.blue + (b_linear.blue - a_linear.blue) * t,
    )
}

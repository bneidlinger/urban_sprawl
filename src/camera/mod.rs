//! Orthographic camera system with zoom, pan, and rotate controls.
//!
//! Includes color grading that adapts to time of day for cinematic atmosphere.

use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomPrefilter},
        experimental::taa::TemporalAntiAliasing,
        motion_blur::MotionBlur,
        tonemapping::Tonemapping,
    },
    input::mouse::MouseMotion,
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::view::ColorGrading,
};

use crate::render::day_night::TimeOfDay;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BloomConfig>()
            .init_resource::<TaaConfig>()
            .init_resource::<TonemappingConfig>()
            .init_resource::<ColorGradingConfig>()
            .init_resource::<MotionBlurConfig>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (
                    camera_zoom,
                    camera_pan,
                    camera_rotate,
                    update_bloom_intensity,
                    update_tonemapping,
                    update_color_grading,
                    update_motion_blur,
                    color_grading_controls,
                ),
            );
    }
}

/// Configuration for bloom effect.
#[derive(Resource)]
pub struct BloomConfig {
    /// Whether bloom is enabled.
    pub enabled: bool,
    /// Base bloom intensity (scaled by night factor).
    pub intensity: f32,
    /// Bloom threshold (luminance above this glows).
    pub threshold: f32,
    /// How soft the threshold is.
    pub threshold_softness: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            threshold: 0.8,
            threshold_softness: 0.3,
        }
    }
}

/// Configuration for Temporal Anti-Aliasing.
#[derive(Resource)]
pub struct TaaConfig {
    /// Whether TAA is enabled.
    /// Note: TAA may cause ghosting artifacts with fast-moving objects.
    pub enabled: bool,
}

impl Default for TaaConfig {
    fn default() -> Self {
        Self {
            // TAA requires PerspectiveProjection - disabled for orthographic camera
            enabled: false,
        }
    }
}

/// Configuration for tonemapping.
#[derive(Resource)]
pub struct TonemappingConfig {
    /// Current tonemapping mode.
    pub mode: TonemappingMode,
}

/// Available tonemapping modes.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TonemappingMode {
    /// TonyMcMapface - good balance of color and contrast
    #[default]
    TonyMcMapface,
    /// AgX - industry standard, handles bright lights naturally
    AgX,
    /// ACES Fitted - filmic look
    AcesFitted,
    /// Reinhard - classic tonemapping
    Reinhard,
    /// No tonemapping (raw HDR values)
    None,
}

impl Default for TonemappingConfig {
    fn default() -> Self {
        Self {
            mode: TonemappingMode::TonyMcMapface,
        }
    }
}

/// Color grading configuration for cinematic look.
#[derive(Resource)]
pub struct ColorGradingConfig {
    /// Whether color grading is enabled.
    pub enabled: bool,
    /// Current color preset.
    pub preset: ColorPreset,
    /// Whether to auto-adjust based on time of day.
    pub auto_time_of_day: bool,
    /// Base exposure adjustment (-2.0 to 2.0).
    pub exposure: f32,
    /// Saturation multiplier (0.0 to 2.0).
    pub saturation: f32,
    /// Contrast adjustment (0.5 to 2.0).
    pub contrast: f32,
}

impl Default for ColorGradingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: ColorPreset::Natural,
            auto_time_of_day: true,
            exposure: 0.0,
            saturation: 1.0,
            contrast: 1.0,
        }
    }
}

/// Color presets for different moods.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorPreset {
    /// Natural colors with subtle enhancements.
    #[default]
    Natural,
    /// Warm sunset/golden hour tones.
    WarmSunset,
    /// Cool twilight/blue hour tones.
    CoolTwilight,
    /// Vibrant, saturated colors.
    Vibrant,
    /// Desaturated, moody look.
    Noir,
    /// Retro/vintage film look.
    Vintage,
}

/// Configuration for motion blur effect.
#[derive(Resource)]
pub struct MotionBlurConfig {
    /// Whether motion blur is enabled.
    pub enabled: bool,
    /// Shutter angle (0.0 to 1.0). 0.5 = 180 degrees (cinematic).
    pub shutter_angle: f32,
    /// Number of samples for quality (1-4 recommended).
    pub samples: u32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shutter_angle: 0.3, // Subtle motion blur
            samples: 2,
        }
    }
}

/// Marker component for the main isometric camera.
#[derive(Component)]
pub struct IsometricCamera {
    pub zoom: f32,
    pub rotation: f32,
}

impl Default for IsometricCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

fn setup_camera(
    mut commands: Commands,
    bloom_config: Res<BloomConfig>,
    taa_config: Res<TaaConfig>,
    tonemap_config: Res<TonemappingConfig>,
    motion_blur_config: Res<MotionBlurConfig>,
) {
    // Standard isometric angle: ~35.264 degrees (arctan(1/sqrt(2)))
    let iso_angle = 35.264_f32.to_radians();
    let distance = 500.0;

    // Determine which tonemapping to use
    let tonemapping = match tonemap_config.mode {
        TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
        TonemappingMode::AgX => Tonemapping::AgX,
        TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
        TonemappingMode::Reinhard => Tonemapping::Reinhard,
        TonemappingMode::None => Tonemapping::None,
    };

    let camera_entity = commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true, // Required for bloom
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: 0.1,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(distance, distance * iso_angle.tan(), distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgba(0.6, 0.7, 0.8, 0.85),
            falloff: FogFalloff::Exponential { density: 0.0015 },
            directional_light_color: Color::srgba(1.0, 0.8, 0.6, 0.3),
            directional_light_exponent: 12.0,
        },
        // Bloom for glowing lights at night
        Bloom {
            intensity: bloom_config.intensity,
            low_frequency_boost: 0.5,
            low_frequency_boost_curvature: 0.7,
            high_pass_frequency: 0.8,
            prefilter: BloomPrefilter {
                threshold: bloom_config.threshold,
                threshold_softness: bloom_config.threshold_softness,
            },
            composite_mode: bevy::core_pipeline::bloom::BloomCompositeMode::Additive,
            ..default()
        },
        // Tonemapping for HDR
        tonemapping,
        // Color grading for cinematic look
        ColorGrading::default(),
        // Motion blur for camera movement
        MotionBlur {
            shutter_angle: motion_blur_config.shutter_angle,
            samples: motion_blur_config.samples,
        },
        IsometricCamera::default(),
    )).id();

    // Add TAA if enabled
    if taa_config.enabled {
        commands.entity(camera_entity).insert(TemporalAntiAliasing::default());
        info!("TAA enabled on camera");
    }
}

fn camera_zoom(
    mut query: Query<(&mut Projection, &mut IsometricCamera)>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
) {
    let scroll: f32 = scroll_events.read().map(|e| e.y).sum();
    if scroll == 0.0 {
        return;
    }

    for (mut projection, mut iso_cam) in &mut query {
        iso_cam.zoom = (iso_cam.zoom - scroll * 0.1).clamp(0.1, 10.0);
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = iso_cam.zoom * 0.1;
        }
    }
}

fn camera_pan(
    mut query: Query<(&mut Transform, &IsometricCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;
    let speed = 100.0;

    // Keyboard panning
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        let delta = direction.normalize() * speed * time.delta_secs();
        for (mut transform, _) in &mut query {
            transform.translation += delta;
        }
    }

    // Mouse panning (middle button or right button drag)
    if mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right) {
        let mut mouse_delta = Vec2::ZERO;
        for event in mouse_motion.read() {
            mouse_delta += event.delta;
        }

        if mouse_delta != Vec2::ZERO {
            for (mut transform, iso_cam) in &mut query {
                // Scale pan speed based on zoom level
                let pan_speed = iso_cam.zoom * 0.5;
                // Direct mapping: mouse X → world X, mouse Y → world Z
                // Negative signs for "grab and drag" feel (drag right = view moves right)
                let world_delta = Vec3::new(
                    -mouse_delta.x * pan_speed,
                    0.0,
                    -mouse_delta.y * pan_speed,
                );
                transform.translation += world_delta;
            }
        }
    } else {
        // Clear any pending mouse motion events when not panning
        mouse_motion.clear();
    }
}

fn camera_rotate(
    mut query: Query<(&mut Transform, &mut IsometricCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let rotation_speed = 1.0;
    let mut rotation_delta = 0.0;

    if keys.pressed(KeyCode::KeyQ) {
        rotation_delta -= rotation_speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        rotation_delta += rotation_speed * time.delta_secs();
    }

    if rotation_delta != 0.0 {
        for (mut transform, mut iso_cam) in &mut query {
            iso_cam.rotation += rotation_delta;
            let target = Vec3::ZERO; // TODO: Track actual look-at target
            transform.rotate_around(target, Quat::from_rotation_y(rotation_delta));
        }
    }
}

/// Update bloom intensity based on time of day.
/// Bloom is more intense at night when lights are glowing.
fn update_bloom_intensity(
    tod: Option<Res<TimeOfDay>>,
    config: Res<BloomConfig>,
    mut bloom_query: Query<&mut Bloom>,
) {
    let Some(tod) = tod else { return };

    if !config.enabled {
        for mut bloom in &mut bloom_query {
            bloom.intensity = 0.0;
        }
        return;
    }

    let hour = tod.hour();

    // Calculate night factor (0.0 = day, 1.0 = night)
    let night_factor = if hour >= 5.0 && hour <= 7.0 {
        // Dawn - transitioning from night
        1.0 - (hour - 5.0) / 2.0
    } else if hour >= 17.0 && hour <= 19.0 {
        // Dusk - transitioning to night
        (hour - 17.0) / 2.0
    } else if hour > 7.0 && hour < 17.0 {
        // Day - minimal bloom
        0.1
    } else {
        // Night - full bloom
        1.0
    };

    // Apply bloom with night scaling
    // During the day, we want subtle bloom; at night, more pronounced
    let intensity = config.intensity * (0.3 + 0.7 * night_factor);

    for mut bloom in &mut bloom_query {
        bloom.intensity = intensity;
    }
}

/// Update tonemapping based on config changes.
fn update_tonemapping(
    config: Res<TonemappingConfig>,
    mut tonemapping_query: Query<&mut Tonemapping>,
) {
    if !config.is_changed() {
        return;
    }

    let new_tonemapping = match config.mode {
        TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
        TonemappingMode::AgX => Tonemapping::AgX,
        TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
        TonemappingMode::Reinhard => Tonemapping::Reinhard,
        TonemappingMode::None => Tonemapping::None,
    };

    for mut tonemapping in &mut tonemapping_query {
        *tonemapping = new_tonemapping;
    }
}

/// Update color grading based on time of day and preset.
fn update_color_grading(
    tod: Option<Res<TimeOfDay>>,
    config: Res<ColorGradingConfig>,
    mut grading_query: Query<&mut ColorGrading>,
) {
    if !config.enabled {
        for mut grading in &mut grading_query {
            // Reset to neutral
            grading.global.exposure = 0.0;
            grading.global.post_saturation = 1.0;
        }
        return;
    }

    let hour = tod.as_ref().map(|t| t.hour()).unwrap_or(12.0);

    // Get base values from preset
    let (base_exposure, base_saturation, shadows_color, highlights_color) = match config.preset {
        ColorPreset::Natural => (0.0, 1.0, Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
        ColorPreset::WarmSunset => (0.1, 1.1, Vec3::new(1.1, 0.95, 0.85), Vec3::new(1.2, 1.0, 0.8)),
        ColorPreset::CoolTwilight => {
            (-0.1, 0.95, Vec3::new(0.9, 0.95, 1.1), Vec3::new(0.85, 0.9, 1.15))
        }
        ColorPreset::Vibrant => (0.05, 1.3, Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
        ColorPreset::Noir => (-0.2, 0.4, Vec3::new(0.95, 0.95, 1.0), Vec3::new(1.0, 1.0, 0.95)),
        ColorPreset::Vintage => {
            (0.0, 0.85, Vec3::new(1.1, 1.0, 0.9), Vec3::new(1.0, 0.95, 0.85))
        }
    };

    // Time-of-day adjustments (if enabled)
    let (tod_exposure, tod_saturation, tod_warmth) = if config.auto_time_of_day {
        if hour >= 5.0 && hour <= 7.0 {
            // Dawn - warm, slightly desaturated
            let t = (hour - 5.0) / 2.0;
            (-0.1 + t * 0.15, 0.9 + t * 0.1, 1.1 - t * 0.1)
        } else if hour >= 17.0 && hour <= 19.0 {
            // Dusk - golden hour warmth
            let t = (hour - 17.0) / 2.0;
            (0.05 - t * 0.15, 1.0 - t * 0.1, 1.0 + t * 0.15)
        } else if hour > 19.0 || hour < 5.0 {
            // Night - cool, slightly desaturated
            (-0.15, 0.85, 0.9)
        } else {
            // Day - neutral
            (0.0, 1.0, 1.0)
        }
    } else {
        (0.0, 1.0, 1.0)
    };

    // Combine preset + ToD + user adjustments
    let final_exposure = base_exposure + tod_exposure + config.exposure;
    let final_saturation = base_saturation * tod_saturation * config.saturation;

    for mut grading in &mut grading_query {
        grading.global.exposure = final_exposure;
        grading.global.post_saturation = final_saturation;

        // Apply shadow/highlight color tints with warmth adjustment
        let warmth_tint = Vec3::new(tod_warmth, 1.0, 2.0 - tod_warmth);
        let final_shadows = shadows_color * warmth_tint;
        let final_highlights = highlights_color * warmth_tint;

        grading.shadows.saturation = final_shadows.x.max(final_shadows.y).max(final_shadows.z);
        grading.highlights.saturation =
            final_highlights.x.max(final_highlights.y).max(final_highlights.z);
    }
}

/// Keyboard controls for color grading presets.
fn color_grading_controls(keyboard: Res<ButtonInput<KeyCode>>, mut config: ResMut<ColorGradingConfig>) {
    // G: Cycle through presets
    if keyboard.just_pressed(KeyCode::KeyG) {
        config.preset = match config.preset {
            ColorPreset::Natural => ColorPreset::WarmSunset,
            ColorPreset::WarmSunset => ColorPreset::CoolTwilight,
            ColorPreset::CoolTwilight => ColorPreset::Vibrant,
            ColorPreset::Vibrant => ColorPreset::Noir,
            ColorPreset::Noir => ColorPreset::Vintage,
            ColorPreset::Vintage => ColorPreset::Natural,
        };
        info!("Color preset: {:?}", config.preset);
    }

    // Shift+G: Toggle color grading
    if keyboard.just_pressed(KeyCode::KeyG)
        && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight))
    {
        config.enabled = !config.enabled;
        info!(
            "Color grading: {}",
            if config.enabled { "ON" } else { "OFF" }
        );
    }

    // T: Toggle time-of-day auto adjustment
    if keyboard.just_pressed(KeyCode::KeyT)
        && (keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight))
    {
        config.auto_time_of_day = !config.auto_time_of_day;
        info!(
            "Auto time-of-day grading: {}",
            if config.auto_time_of_day { "ON" } else { "OFF" }
        );
    }
}

/// Update motion blur settings based on config.
fn update_motion_blur(
    config: Res<MotionBlurConfig>,
    mut commands: Commands,
    motion_blur_query: Query<(Entity, Option<&MotionBlur>), With<IsometricCamera>>,
) {
    if !config.is_changed() {
        return;
    }

    for (entity, existing_blur) in &motion_blur_query {
        if config.enabled {
            // Add or update motion blur
            commands.entity(entity).insert(MotionBlur {
                shutter_angle: config.shutter_angle,
                samples: config.samples,
            });
        } else if existing_blur.is_some() {
            // Remove motion blur
            commands.entity(entity).remove::<MotionBlur>();
        }
    }
}

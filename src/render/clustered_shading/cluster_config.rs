//! Configuration for Bevy's clustered forward rendering.
//!
//! Bevy uses clustered forward rendering which subdivides the view frustum
//! into a 3D grid of clusters. Each cluster maintains a list of lights that
//! affect it, enabling efficient many-light rendering.

use bevy::prelude::*;

/// Configuration for the clustered lighting system.
#[derive(Resource)]
pub struct ClusterConfig {
    /// Maximum number of point lights in the scene.
    /// Bevy's default is 256, we increase for city simulation.
    pub max_point_lights: usize,

    /// Radius for street lamp point lights (in world units).
    pub street_lamp_radius: f32,

    /// Intensity for street lamp point lights (lumens).
    pub street_lamp_intensity: f32,

    /// Color for street lamps (warm white).
    pub street_lamp_color: Color,

    /// Radius for traffic light point lights.
    pub traffic_light_radius: f32,

    /// Intensity for traffic lights.
    pub traffic_light_intensity: f32,

    /// Radius for window lights.
    pub window_light_radius: f32,

    /// Intensity for window lights.
    pub window_light_intensity: f32,

    /// Whether to enable shadows for point lights (expensive!).
    pub point_light_shadows: bool,

    /// Shadow map resolution for point lights (if enabled).
    pub point_light_shadow_resolution: u32,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            // Bevy can handle thousands of lights with clustering
            max_point_lights: 4096,

            // Street lamp settings - boosted for dark nights
            street_lamp_radius: 30.0,         // 30m radius for wider pools of light
            street_lamp_intensity: 15000.0,   // Brighter to punch through darkness
            street_lamp_color: Color::srgb(1.0, 0.9, 0.7), // Warm white (3000K-ish)

            // Traffic light settings - boosted
            traffic_light_radius: 18.0,
            traffic_light_intensity: 4000.0,

            // Window light settings - boosted for visibility
            window_light_radius: 12.0,
            window_light_intensity: 1200.0,

            // Shadows are expensive for many lights, disable by default
            point_light_shadows: false,
            point_light_shadow_resolution: 512,
        }
    }
}

impl ClusterConfig {
    /// Create a PointLight for a street lamp.
    pub fn create_street_lamp_light(&self) -> PointLight {
        PointLight {
            color: self.street_lamp_color,
            intensity: 0.0, // Will be set by DynamicCityLight system
            range: self.street_lamp_radius,
            radius: 0.5, // Physical size of the light source
            shadows_enabled: self.point_light_shadows,
            ..default()
        }
    }

    /// Create a PointLight for a traffic light.
    pub fn create_traffic_light(&self, color: Color) -> PointLight {
        PointLight {
            color,
            intensity: self.traffic_light_intensity,
            range: self.traffic_light_radius,
            radius: 0.12,
            shadows_enabled: false,
            ..default()
        }
    }

    /// Create a PointLight for a window.
    pub fn create_window_light(&self, color: Color) -> PointLight {
        PointLight {
            color,
            intensity: 0.0, // Will be set by DynamicCityLight system
            range: self.window_light_radius,
            radius: 0.3,
            shadows_enabled: false,
            ..default()
        }
    }
}

/// Helper to create warm white variations for street lamps.
pub fn street_lamp_color_variation(seed: u32) -> Color {
    // Slight variations in color temperature
    let r = 1.0;
    let g = 0.85 + (seed % 10) as f32 * 0.01;
    let b = 0.6 + (seed % 20) as f32 * 0.01;
    Color::srgb(r, g, b)
}

/// Colors for traffic lights.
pub mod traffic_colors {
    use bevy::prelude::Color;

    pub const RED: Color = Color::srgb(1.0, 0.1, 0.1);
    pub const YELLOW: Color = Color::srgb(1.0, 0.85, 0.1);
    pub const GREEN: Color = Color::srgb(0.1, 1.0, 0.2);
}

/// Colors for window lights (interior illumination).
pub mod window_colors {
    use bevy::prelude::Color;

    /// Warm incandescent-like
    pub const WARM: Color = Color::srgb(1.0, 0.85, 0.6);
    /// Cool fluorescent-like
    pub const COOL: Color = Color::srgb(0.9, 0.95, 1.0);
    /// TV/screen blue glow
    pub const SCREEN: Color = Color::srgb(0.6, 0.7, 1.0);
    /// Neutral white
    pub const NEUTRAL: Color = Color::srgb(1.0, 0.95, 0.9);
}

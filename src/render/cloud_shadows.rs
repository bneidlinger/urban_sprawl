//! Cloud shadows that drift across the city landscape.
//!
//! Uses a procedural noise shader projected onto a large plane above the terrain.

use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

use super::day_night::TimeOfDay;

pub struct CloudShadowsPlugin;

impl Plugin for CloudShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CloudShadowMaterial>::default())
            .init_resource::<CloudShadowConfig>()
            .init_resource::<CloudScrollState>()
            .add_systems(Startup, spawn_cloud_shadow_plane)
            .add_systems(Update, update_cloud_shadows);
    }
}

/// Configuration for cloud shadow appearance and movement.
#[derive(Resource)]
pub struct CloudShadowConfig {
    /// Size of the shadow plane (should cover entire city).
    pub plane_size: f32,
    /// Height above ground for the shadow plane.
    pub plane_height: f32,
    /// Wind direction (normalized).
    pub wind_direction: Vec2,
    /// Wind speed in world units per second.
    pub wind_speed: f32,
    /// Noise scale (smaller = larger clouds).
    pub noise_scale: f32,
    /// Cloud coverage (0.0 = clear sky, 1.0 = overcast).
    pub coverage: f32,
    /// Edge softness for cloud shadows.
    pub softness: f32,
    /// Maximum shadow opacity during day.
    pub max_opacity: f32,
}

impl Default for CloudShadowConfig {
    fn default() -> Self {
        Self {
            plane_size: 800.0,
            plane_height: 0.5, // Just above ground to avoid z-fighting
            wind_direction: Vec2::new(1.0, 0.3).normalize(),
            wind_speed: 15.0,
            noise_scale: 0.008,
            coverage: 0.45,
            softness: 0.15,
            max_opacity: 0.35,
        }
    }
}

/// Custom material for cloud shadows using procedural noise.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct CloudShadowMaterial {
    #[uniform(0)]
    pub scroll_offset: Vec2,
    #[uniform(0)]
    pub opacity: f32,
    #[uniform(0)]
    pub scale: f32,
    #[uniform(0)]
    pub coverage: f32,
    #[uniform(0)]
    pub softness: f32,
    #[uniform(0)]
    pub _padding: Vec2,
}

impl Material for CloudShadowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cloud_shadows.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

impl Default for CloudShadowMaterial {
    fn default() -> Self {
        Self {
            scroll_offset: Vec2::ZERO,
            opacity: 0.3,
            scale: 0.008,
            coverage: 0.45,
            softness: 0.15,
            _padding: Vec2::ZERO,
        }
    }
}

/// Marker component for the cloud shadow plane entity.
#[derive(Component)]
pub struct CloudShadowPlane;

/// Tracks the accumulated scroll offset for cloud movement.
#[derive(Resource, Default)]
pub struct CloudScrollState {
    pub offset: Vec2,
}

fn spawn_cloud_shadow_plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudShadowMaterial>>,
    config: Res<CloudShadowConfig>,
) {
    // Create a large plane mesh
    let plane_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(config.plane_size, config.plane_size),
    );

    // Create the cloud shadow material
    let material = materials.add(CloudShadowMaterial {
        scroll_offset: Vec2::ZERO,
        opacity: config.max_opacity,
        scale: config.noise_scale,
        coverage: config.coverage,
        softness: config.softness,
        _padding: Vec2::ZERO,
    });

    // Spawn the shadow plane slightly above the terrain
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, config.plane_height, 0.0),
        CloudShadowPlane,
    ));

    info!(
        "Cloud shadow plane spawned at height {}",
        config.plane_height
    );
}

fn update_cloud_shadows(
    time: Res<Time>,
    time_of_day: Res<TimeOfDay>,
    config: Res<CloudShadowConfig>,
    mut scroll_state: ResMut<CloudScrollState>,
    mut materials: ResMut<Assets<CloudShadowMaterial>>,
    query: Query<&MeshMaterial3d<CloudShadowMaterial>, With<CloudShadowPlane>>,
) {
    // Update scroll offset based on wind and wrap periodically to avoid precision loss
    scroll_state.offset += config.wind_direction * config.wind_speed * time.delta_secs();

    // Keep the offset bounded so very long sessions don't accumulate huge UV values
    let wrap_distance = config.plane_size * 4.0;
    scroll_state.offset = Vec2::new(
        scroll_state.offset.x.rem_euclid(wrap_distance),
        scroll_state.offset.y.rem_euclid(wrap_distance),
    );

    // Calculate opacity based on time of day (fade out at night)
    let day_factor = time_of_day.transition_factor();
    let target_opacity = config.max_opacity * day_factor;

    // Update material uniforms
    for material_handle in query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            material.scroll_offset = scroll_state.offset;
            material.opacity = target_opacity;
        }
    }
}

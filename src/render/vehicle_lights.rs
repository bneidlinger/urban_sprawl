//! Vehicle headlights and taillights.
//!
//! Adds headlights to moving vehicles and occasional headlights to parked cars.
//! Headlights are bright white, taillights are dim red.

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::render::clustered_shading::{ClusterConfig, DynamicCityLight};
use crate::render::day_night::TimeOfDay;
use crate::render::parked_cars::ParkedCar;
use crate::simulation::vehicles::MovingVehicle;

pub struct VehicleLightsPlugin;

impl Plugin for VehicleLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleLightConfig>()
            .init_resource::<VehicleLightsSpawned>()
            .add_systems(Update, spawn_vehicle_lights.run_if(should_spawn_lights))
            .add_systems(Update, update_moving_vehicle_lights);
    }
}

/// Marker resource to prevent vehicle lights from spawning multiple times.
#[derive(Resource, Default)]
pub struct VehicleLightsSpawned(pub bool);

fn should_spawn_lights(
    parked_query: Query<&ParkedCar>,
    moving_query: Query<&MovingVehicle>,
    spawned: Res<VehicleLightsSpawned>,
) -> bool {
    (!parked_query.is_empty() || !moving_query.is_empty()) && !spawned.0
}

/// Component marking a vehicle headlight.
#[derive(Component)]
pub struct VehicleHeadlight {
    /// Entity of the vehicle this light belongs to
    pub vehicle: Entity,
    /// Is this a moving vehicle (vs parked)
    pub is_moving: bool,
    /// Left or right headlight
    pub is_left: bool,
}

/// Component marking a vehicle taillight.
#[derive(Component)]
pub struct VehicleTaillight {
    /// Entity of the vehicle this light belongs to
    pub vehicle: Entity,
}

/// Configuration for vehicle lights.
#[derive(Resource)]
pub struct VehicleLightConfig {
    pub seed: u64,
    /// Probability that a parked car has its lights left on
    pub parked_lights_on_probability: f32,
    /// Headlight intensity (lumens)
    pub headlight_intensity: f32,
    /// Taillight intensity (lumens)
    pub taillight_intensity: f32,
    /// Headlight range (meters)
    pub headlight_range: f32,
    /// Taillight range (meters)
    pub taillight_range: f32,
    /// Offset from car center to headlight position (forward)
    pub headlight_forward_offset: f32,
    /// Offset from car center to headlight position (sideways)
    pub headlight_side_offset: f32,
    /// Height of headlights above ground
    pub headlight_height: f32,
}

impl Default for VehicleLightConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            parked_lights_on_probability: 0.05, // 5% of parked cars have lights on
            headlight_intensity: 15000.0,        // Bright headlights
            taillight_intensity: 1500.0,         // Dimmer taillights
            headlight_range: 25.0,               // 25m range
            taillight_range: 8.0,                // 8m range
            headlight_forward_offset: 2.0,       // 2m forward from center
            headlight_side_offset: 0.7,          // 0.7m left/right of center
            headlight_height: 0.7,               // 0.7m above ground
        }
    }
}

fn spawn_vehicle_lights(
    mut commands: Commands,
    config: Res<VehicleLightConfig>,
    cluster_config: Res<ClusterConfig>,
    parked_query: Query<(Entity, &Transform), With<ParkedCar>>,
    moving_query: Query<(Entity, &Transform), With<MovingVehicle>>,
    mut spawned: ResMut<VehicleLightsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning vehicle lights...");

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut parked_headlight_count = 0;
    let mut moving_headlight_count = 0;

    // Headlight color (warm white, slightly yellow)
    let headlight_color = Color::srgb(1.0, 0.95, 0.85);
    // Taillight color (red)
    let taillight_color = Color::srgb(1.0, 0.1, 0.05);

    // Track which parked car entities we've already processed
    // (ParkedCar is spawned multiple times per car - body, cabin, wheels)
    let mut processed_parked: std::collections::HashSet<i32> = std::collections::HashSet::new();

    // Add lights to parked cars (only some have lights on)
    for (entity, transform) in parked_query.iter() {
        // Use position as a hash to avoid duplicate lights per car
        let pos_hash = (transform.translation.x as i32) ^ (transform.translation.z as i32 * 1000);
        if processed_parked.contains(&pos_hash) {
            continue;
        }
        processed_parked.insert(pos_hash);

        // Only some parked cars have lights on
        if rng.gen::<f32>() > config.parked_lights_on_probability {
            continue;
        }

        // Get car orientation
        let forward = transform.forward();
        let right = transform.right();
        let pos = transform.translation;

        // Spawn headlights (left and right)
        for (is_left, side_mult) in [(true, 1.0_f32), (false, -1.0_f32)] {
            let light_pos = pos
                + forward * config.headlight_forward_offset
                + right * config.headlight_side_offset * side_mult;

            commands.spawn((
                PointLight {
                    color: headlight_color,
                    intensity: 0.0, // Managed by DynamicCityLight
                    range: config.headlight_range,
                    radius: 0.1,
                    shadows_enabled: false, // Too many lights for shadows
                    ..default()
                },
                Transform::from_xyz(light_pos.x, config.headlight_height, light_pos.z),
                DynamicCityLight::street_lamp(config.headlight_intensity * 0.3), // Dimmer for parked
                VehicleHeadlight {
                    vehicle: entity,
                    is_moving: false,
                    is_left,
                },
            ));

            parked_headlight_count += 1;
        }

        // Spawn taillights (left and right, at rear)
        for side_mult in [1.0_f32, -1.0] {
            let light_pos = pos
                - forward * config.headlight_forward_offset
                + right * config.headlight_side_offset * side_mult;

            commands.spawn((
                PointLight {
                    color: taillight_color,
                    intensity: 0.0,
                    range: config.taillight_range,
                    radius: 0.05,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(light_pos.x, config.headlight_height, light_pos.z),
                DynamicCityLight::street_lamp(config.taillight_intensity * 0.3),
                VehicleTaillight { vehicle: entity },
            ));
        }
    }

    // Add lights to moving vehicles (all have lights)
    for (entity, transform) in moving_query.iter() {
        let forward = transform.forward();
        let right = transform.right();
        let pos = transform.translation;

        // Spawn headlights
        for (is_left, side_mult) in [(true, 1.0_f32), (false, -1.0_f32)] {
            let light_pos = pos
                + forward * config.headlight_forward_offset
                + right * config.headlight_side_offset * side_mult;

            commands.spawn((
                PointLight {
                    color: headlight_color,
                    intensity: 0.0,
                    range: config.headlight_range,
                    radius: 0.1,
                    shadows_enabled: cluster_config.point_light_shadows,
                    ..default()
                },
                Transform::from_xyz(light_pos.x, config.headlight_height, light_pos.z),
                DynamicCityLight::street_lamp(config.headlight_intensity),
                VehicleHeadlight {
                    vehicle: entity,
                    is_moving: true,
                    is_left,
                },
            ));

            moving_headlight_count += 1;
        }

        // Spawn taillights
        for side_mult in [1.0_f32, -1.0] {
            let light_pos = pos
                - forward * config.headlight_forward_offset
                + right * config.headlight_side_offset * side_mult;

            commands.spawn((
                PointLight {
                    color: taillight_color,
                    intensity: 0.0,
                    range: config.taillight_range,
                    radius: 0.05,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(light_pos.x, config.headlight_height, light_pos.z),
                DynamicCityLight::street_lamp(config.taillight_intensity),
                VehicleTaillight { vehicle: entity },
            ));
        }
    }

    info!(
        "Spawned vehicle lights: {} parked headlights, {} moving headlights",
        parked_headlight_count, moving_headlight_count
    );
}

/// Update moving vehicle light positions to follow their vehicles.
fn update_moving_vehicle_lights(
    config: Res<VehicleLightConfig>,
    vehicle_query: Query<&Transform, With<MovingVehicle>>,
    mut headlight_query: Query<(&VehicleHeadlight, &mut Transform), Without<MovingVehicle>>,
    mut taillight_query: Query<
        (&VehicleTaillight, &mut Transform),
        (Without<MovingVehicle>, Without<VehicleHeadlight>),
    >,
) {
    // Update headlight positions
    for (headlight, mut light_transform) in headlight_query.iter_mut() {
        if !headlight.is_moving {
            continue;
        }

        if let Ok(vehicle_transform) = vehicle_query.get(headlight.vehicle) {
            let forward = vehicle_transform.forward();
            let right = vehicle_transform.right();
            let pos = vehicle_transform.translation;
            let side_mult = if headlight.is_left { 1.0 } else { -1.0 };

            let light_pos = pos
                + forward * config.headlight_forward_offset
                + right * config.headlight_side_offset * side_mult;

            light_transform.translation = Vec3::new(light_pos.x, config.headlight_height, light_pos.z);
        }
    }

    // Update taillight positions
    for (taillight, mut light_transform) in taillight_query.iter_mut() {
        if let Ok(vehicle_transform) = vehicle_query.get(taillight.vehicle) {
            let forward = vehicle_transform.forward();
            let right = vehicle_transform.right();
            let pos = vehicle_transform.translation;

            // Taillights are at the rear - we need to figure out which side
            // For simplicity, just place them at rear center offset
            let light_pos = pos - forward * config.headlight_forward_offset;

            light_transform.translation = Vec3::new(light_pos.x, config.headlight_height, light_pos.z);
        }
    }
}

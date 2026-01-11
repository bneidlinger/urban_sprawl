//! Train cars moving on elevated rail tracks.
//!
//! Spawns train sets that travel along the elevated rail line,
//! stopping briefly at stations.

use bevy::prelude::*;
use std::f32::consts::PI;

use crate::render::elevated_rail::{ElevatedRailConfig, ElevatedRailSpawned, RailLine};

pub struct TrainCarsPlugin;

impl Plugin for TrainCarsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrainConfig>()
            .init_resource::<TrainsSpawned>()
            .add_systems(Update, spawn_trains.run_if(should_spawn_trains))
            .add_systems(Update, update_train_movement.run_if(trains_exist));
    }
}

#[derive(Resource, Default)]
pub struct TrainsSpawned(pub bool);

fn should_spawn_trains(
    rail_spawned: Res<ElevatedRailSpawned>,
    trains_spawned: Res<TrainsSpawned>,
) -> bool {
    rail_spawned.0 && !trains_spawned.0
}

fn trains_exist(trains: Query<&Train>) -> bool {
    !trains.is_empty()
}

#[derive(Resource)]
pub struct TrainConfig {
    /// Number of train sets to spawn.
    pub train_count: usize,
    /// Number of cars per train.
    pub cars_per_train: usize,
    /// Length of each car.
    pub car_length: f32,
    /// Width of each car.
    pub car_width: f32,
    /// Height of each car.
    pub car_height: f32,
    /// Gap between cars.
    pub car_gap: f32,
    /// Train speed (units per second).
    pub speed: f32,
    /// Time stopped at stations (seconds).
    pub station_dwell: f32,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            train_count: 2,
            cars_per_train: 4,
            car_length: 12.0,
            car_width: 2.8,
            car_height: 3.2,
            car_gap: 0.5,
            speed: 15.0,
            station_dwell: 5.0,
        }
    }
}

/// Train entity (the lead car).
#[derive(Component)]
pub struct Train {
    /// Current position along the rail line (0.0 to 1.0).
    pub progress: f32,
    /// Current speed multiplier (0 when stopped, 1 normally).
    pub speed_factor: f32,
    /// Direction of travel (1 = forward, -1 = reverse).
    pub direction: f32,
    /// Time remaining at current station (if stopped).
    pub station_timer: f32,
    /// Whether currently at a station.
    pub at_station: bool,
}

/// Individual train car (follows the lead car).
#[derive(Component)]
pub struct TrainCar {
    /// Index in the train (0 = lead car).
    pub car_index: usize,
    /// Reference to the lead train entity.
    pub train_entity: Entity,
}

/// Marker for train windows.
#[derive(Component)]
pub struct TrainWindow;

fn spawn_trains(
    mut commands: Commands,
    config: Res<TrainConfig>,
    rail_config: Res<ElevatedRailConfig>,
    rail_line: Res<RailLine>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<TrainsSpawned>,
) {
    spawned.0 = true;

    if rail_line.waypoints.len() < 2 {
        info!("No rail line for trains");
        return;
    }

    // Materials
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.88),
        metallic: 0.3,
        perceptual_roughness: 0.4,
        ..default()
    });

    let accent_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.7),
        metallic: 0.5,
        perceptual_roughness: 0.3,
        ..default()
    });

    let window_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.25, 0.35, 0.7),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.2,
        perceptual_roughness: 0.1,
        ..default()
    });

    let roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.62),
        perceptual_roughness: 0.7,
        ..default()
    });

    // Meshes
    let body_mesh = meshes.add(Cuboid::new(
        config.car_length,
        config.car_height * 0.7,
        config.car_width,
    ));
    let roof_mesh = meshes.add(Cuboid::new(
        config.car_length - 0.2,
        config.car_height * 0.15,
        config.car_width - 0.1,
    ));
    let window_mesh = meshes.add(Cuboid::new(1.2, config.car_height * 0.3, 0.05));
    let stripe_mesh = meshes.add(Cuboid::new(config.car_length, 0.3, config.car_width + 0.02));
    let ac_mesh = meshes.add(Cuboid::new(2.0, 0.3, 1.0));

    // Spawn trains at evenly spaced positions along the line
    for train_idx in 0..config.train_count {
        let start_progress = train_idx as f32 / config.train_count as f32;
        let direction = if train_idx % 2 == 0 { 1.0 } else { -1.0 };

        // Calculate initial position
        let (pos, rot) = get_rail_position(&rail_line.waypoints, start_progress);
        let train_y = rail_config.track_height + config.car_height / 2.0 + 0.4;

        // Spawn the train (lead car entity with Train component)
        let train_entity = commands
            .spawn((
                Transform::from_xyz(pos.x, train_y, pos.y).with_rotation(rot),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Train {
                    progress: start_progress,
                    speed_factor: 1.0,
                    direction,
                    station_timer: 0.0,
                    at_station: false,
                },
            ))
            .id();

        // Spawn all cars as children of a shared parent logic
        for car_idx in 0..config.cars_per_train {
            let car_offset = car_idx as f32 * (config.car_length + config.car_gap);
            let car_progress = start_progress - (car_offset / get_total_rail_length(&rail_line.waypoints)) * direction;
            let (car_pos, car_rot) = get_rail_position(&rail_line.waypoints, car_progress.rem_euclid(1.0));

            commands
                .spawn((
                    Transform::from_xyz(car_pos.x, train_y, car_pos.y).with_rotation(car_rot),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    TrainCar {
                        car_index: car_idx,
                        train_entity,
                    },
                ))
                .with_children(|parent| {
                    // Main body
                    parent.spawn((
                        Mesh3d(body_mesh.clone()),
                        MeshMaterial3d(body_material.clone()),
                        Transform::IDENTITY,
                    ));

                    // Roof
                    parent.spawn((
                        Mesh3d(roof_mesh.clone()),
                        MeshMaterial3d(roof_material.clone()),
                        Transform::from_xyz(0.0, config.car_height * 0.4, 0.0),
                    ));

                    // Color stripe
                    parent.spawn((
                        Mesh3d(stripe_mesh.clone()),
                        MeshMaterial3d(accent_material.clone()),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    ));

                    // Windows on both sides
                    let window_count = 5;
                    for w in 0..window_count {
                        let x = (w as f32 - (window_count - 1) as f32 / 2.0) * 2.0;
                        // Front side
                        parent.spawn((
                            Mesh3d(window_mesh.clone()),
                            MeshMaterial3d(window_material.clone()),
                            Transform::from_xyz(x, config.car_height * 0.1, config.car_width / 2.0 + 0.03),
                            TrainWindow,
                        ));
                        // Back side
                        parent.spawn((
                            Mesh3d(window_mesh.clone()),
                            MeshMaterial3d(window_material.clone()),
                            Transform::from_xyz(x, config.car_height * 0.1, -config.car_width / 2.0 - 0.03),
                            TrainWindow,
                        ));
                    }

                    // AC unit on roof
                    if car_idx == 0 || car_idx == config.cars_per_train - 1 {
                        parent.spawn((
                            Mesh3d(ac_mesh.clone()),
                            MeshMaterial3d(roof_material.clone()),
                            Transform::from_xyz(0.0, config.car_height * 0.55, 0.0),
                        ));
                    }
                });
        }
    }

    info!(
        "Spawned {} trains with {} cars each",
        config.train_count, config.cars_per_train
    );
}

fn update_train_movement(
    time: Res<Time>,
    config: Res<TrainConfig>,
    rail_config: Res<ElevatedRailConfig>,
    rail_line: Res<RailLine>,
    mut trains: Query<(Entity, &mut Train)>,
    mut cars: Query<(&mut Transform, &TrainCar), Without<Train>>,
) {
    if rail_line.waypoints.len() < 2 {
        return;
    }

    let dt = time.delta_secs();
    let total_length = get_total_rail_length(&rail_line.waypoints);
    let train_y = rail_config.track_height + config.car_height / 2.0 + 0.4;

    for (train_entity, mut train) in trains.iter_mut() {
        // Check if at station
        let near_station = rail_line.stations.iter().any(|&station_idx| {
            let station_progress = station_idx as f32 / (rail_line.waypoints.len() - 1) as f32;
            (train.progress - station_progress).abs() < 0.02
        });

        if near_station && !train.at_station && train.speed_factor > 0.5 {
            // Arriving at station
            train.at_station = true;
            train.station_timer = config.station_dwell;
            train.speed_factor = 0.0;
        }

        if train.at_station {
            train.station_timer -= dt;
            if train.station_timer <= 0.0 {
                train.at_station = false;
                train.speed_factor = 1.0;
            }
        }

        // Move train
        let progress_delta = (config.speed * train.speed_factor * dt) / total_length;
        train.progress += progress_delta * train.direction;

        // Wrap around at ends
        if train.progress > 1.0 {
            train.progress = 1.0;
            train.direction = -1.0;
        } else if train.progress < 0.0 {
            train.progress = 0.0;
            train.direction = 1.0;
        }

        // Update all cars for this train
        for (mut car_transform, train_car) in cars.iter_mut() {
            if train_car.train_entity != train_entity {
                continue;
            }

            let car_offset = train_car.car_index as f32 * (config.car_length + config.car_gap);
            let car_progress = train.progress - (car_offset / total_length) * train.direction;
            let car_progress_clamped = car_progress.rem_euclid(1.0);

            let (pos, rot) = get_rail_position(&rail_line.waypoints, car_progress_clamped);

            car_transform.translation = Vec3::new(pos.x, train_y, pos.y);
            car_transform.rotation = rot;
        }
    }
}

/// Get position and rotation along the rail line.
fn get_rail_position(waypoints: &[Vec2], progress: f32) -> (Vec2, Quat) {
    if waypoints.len() < 2 {
        return (Vec2::ZERO, Quat::IDENTITY);
    }

    let total_length = get_total_rail_length(waypoints);
    let target_dist = progress.clamp(0.0, 1.0) * total_length;
    let mut accumulated = 0.0;

    for window in waypoints.windows(2) {
        let seg_len = window[0].distance(window[1]);
        if accumulated + seg_len >= target_dist {
            let local_t = if seg_len > 0.0 {
                (target_dist - accumulated) / seg_len
            } else {
                0.0
            };
            let pos = window[0].lerp(window[1], local_t);
            let dir = (window[1] - window[0]).normalize_or_zero();
            let angle = dir.y.atan2(dir.x);
            return (pos, Quat::from_rotation_y(-angle + PI / 2.0));
        }
        accumulated += seg_len;
    }

    let last = *waypoints.last().unwrap();
    let dir = if waypoints.len() >= 2 {
        (waypoints[waypoints.len() - 1] - waypoints[waypoints.len() - 2]).normalize_or_zero()
    } else {
        Vec2::X
    };
    let angle = dir.y.atan2(dir.x);
    (last, Quat::from_rotation_y(-angle + PI / 2.0))
}

/// Get total length of the rail line.
fn get_total_rail_length(waypoints: &[Vec2]) -> f32 {
    waypoints
        .windows(2)
        .map(|w| w[0].distance(w[1]))
        .sum::<f32>()
        .max(1.0)
}

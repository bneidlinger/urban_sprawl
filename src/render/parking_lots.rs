//! Surface parking lots near commercial and industrial areas.
//!
//! Spawns parking lots with marked spaces and parked cars.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct ParkingLotsPlugin;

impl Plugin for ParkingLotsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParkingLotConfig>()
            .init_resource::<ParkingLotsSpawned>()
            .add_systems(Update, spawn_parking_lots.run_if(should_spawn_lots));
    }
}

#[derive(Resource, Default)]
pub struct ParkingLotsSpawned(pub bool);

fn should_spawn_lots(
    buildings_spawned: Res<BuildingsSpawned>,
    lots_spawned: Res<ParkingLotsSpawned>,
) -> bool {
    buildings_spawned.0 && !lots_spawned.0
}

/// Parking lot marker component.
#[derive(Component)]
pub struct ParkingLot {
    pub capacity: u32,
    pub occupied: u32,
}

#[derive(Resource)]
pub struct ParkingLotConfig {
    pub seed: u64,
    pub max_lots: usize,
    pub min_spacing: f32,
    pub lot_width: f32,
    pub lot_depth: f32,
    pub space_width: f32,
    pub space_depth: f32,
}

impl Default for ParkingLotConfig {
    fn default() -> Self {
        Self {
            seed: 66666,
            max_lots: 12,
            min_spacing: 60.0,
            lot_width: 20.0,
            lot_depth: 15.0,
            space_width: 2.5,
            space_depth: 5.0,
        }
    }
}

// Car colors for parked cars
const CAR_COLORS: &[(f32, f32, f32)] = &[
    (0.1, 0.1, 0.12),
    (0.9, 0.9, 0.92),
    (0.6, 0.6, 0.65),
    (0.15, 0.15, 0.2),
    (0.5, 0.1, 0.1),
    (0.1, 0.2, 0.4),
    (0.2, 0.25, 0.2),
    (0.4, 0.35, 0.25),
];

fn spawn_parking_lots(
    mut commands: Commands,
    config: Res<ParkingLotConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<ParkingLotsSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut lot_count = 0;
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Collect commercial and industrial building positions
    let mut eligible_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| {
            matches!(
                b.building_type,
                BuildingArchetype::Commercial | BuildingArchetype::Industrial
            )
        })
        .map(|(_, t)| t.translation)
        .collect();

    // Shuffle
    for i in (1..eligible_positions.len()).rev() {
        let j = rng.gen_range(0..=i);
        eligible_positions.swap(i, j);
    }

    // Materials
    let asphalt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.9,
        ..default()
    });

    let marking_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.85),
        perceptual_roughness: 0.7,
        ..default()
    });

    let curb_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.58),
        perceptual_roughness: 0.8,
        ..default()
    });

    // Meshes
    let lot_surface = meshes.add(Cuboid::new(config.lot_width, 0.05, config.lot_depth));
    let line_mesh = meshes.add(Cuboid::new(0.1, 0.02, config.space_depth * 0.8));
    let curb_mesh = meshes.add(Cuboid::new(config.lot_width + 0.4, 0.15, 0.2));
    let car_body_mesh = meshes.add(Cuboid::new(4.0, 1.2 * 0.6, 1.7));
    let car_cabin_mesh = meshes.add(Cuboid::new(2.0, 1.2 * 0.4, 1.5));

    for pos in eligible_positions {
        if lot_count >= config.max_lots {
            break;
        }

        // Offset from building
        let offset = Vec3::new(
            rng.gen_range(-15.0..15.0),
            0.0,
            rng.gen_range(-15.0..15.0),
        );
        let lot_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        // Check spacing
        let too_close = placed_positions
            .iter()
            .any(|p| p.distance(lot_pos) < config.min_spacing);
        if too_close {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen_range(0..4) as f32 * PI / 2.0);
        let spaces_per_row = (config.lot_width / config.space_width).floor() as u32;
        let rows = 2;
        let capacity = spaces_per_row * rows;
        let occupied = rng.gen_range(capacity / 3..capacity);

        commands.spawn((
            Transform::from_translation(lot_pos).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            ParkingLot { capacity, occupied },
        )).with_children(|parent| {
            // Asphalt surface
            parent.spawn((
                Mesh3d(lot_surface.clone()),
                MeshMaterial3d(asphalt_material.clone()),
                Transform::from_xyz(0.0, 0.025, 0.0),
            ));

            // Curbs on front and back
            for z in [-config.lot_depth / 2.0, config.lot_depth / 2.0] {
                parent.spawn((
                    Mesh3d(curb_mesh.clone()),
                    MeshMaterial3d(curb_material.clone()),
                    Transform::from_xyz(0.0, 0.075, z),
                ));
            }

            // Parking space lines
            let start_x = -config.lot_width / 2.0 + config.space_width / 2.0;
            for i in 0..=spaces_per_row {
                let x = start_x + i as f32 * config.space_width - config.space_width / 2.0;
                // Front row lines
                parent.spawn((
                    Mesh3d(line_mesh.clone()),
                    MeshMaterial3d(marking_material.clone()),
                    Transform::from_xyz(x, 0.06, -config.lot_depth / 4.0),
                ));
                // Back row lines
                parent.spawn((
                    Mesh3d(line_mesh.clone()),
                    MeshMaterial3d(marking_material.clone()),
                    Transform::from_xyz(x, 0.06, config.lot_depth / 4.0),
                ));
            }

            // Spawn parked cars in occupied spaces
            let mut cars_placed = 0;
            for row in 0..rows {
                let row_z = if row == 0 {
                    -config.lot_depth / 4.0
                } else {
                    config.lot_depth / 4.0
                };
                let car_rotation = if row == 0 {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_y(PI)
                };

                for space in 0..spaces_per_row {
                    if cars_placed >= occupied {
                        break;
                    }

                    // Random chance to skip (create empty spaces)
                    if rng.gen::<f32>() < 0.3 {
                        continue;
                    }

                    let space_x = start_x + space as f32 * config.space_width;
                    let (r, g, b) = CAR_COLORS[rng.gen_range(0..CAR_COLORS.len())];

                    let car_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(r, g, b),
                        perceptual_roughness: 0.4,
                        metallic: 0.6,
                        ..default()
                    });

                    let window_material = materials.add(StandardMaterial {
                        base_color: Color::srgba(0.1, 0.15, 0.2, 0.8),
                        perceptual_roughness: 0.1,
                        metallic: 0.3,
                        ..default()
                    });

                    // Car body
                    parent.spawn((
                        Mesh3d(car_body_mesh.clone()),
                        MeshMaterial3d(car_material.clone()),
                        Transform::from_xyz(space_x, 0.4, row_z).with_rotation(car_rotation),
                    ));

                    // Car cabin
                    parent.spawn((
                        Mesh3d(car_cabin_mesh.clone()),
                        MeshMaterial3d(window_material),
                        Transform::from_xyz(space_x, 0.8, row_z).with_rotation(car_rotation),
                    ));

                    cars_placed += 1;
                }
            }
        });

        placed_positions.push(lot_pos);
        lot_count += 1;
    }

    info!("Spawned {} parking lots with {} total spaces", lot_count, lot_count * 16);
}

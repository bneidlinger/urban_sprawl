//! Street amenities: phone booths, charging stations, bike racks, café seating.
//!
//! Spawns urban amenities along sidewalks for visual detail.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct StreetAmenitiesPlugin;

impl Plugin for StreetAmenitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetAmenitiesConfig>()
            .init_resource::<StreetAmenitiesSpawned>()
            .add_systems(Update, spawn_street_amenities.run_if(should_spawn_amenities));
    }
}

#[derive(Resource, Default)]
pub struct StreetAmenitiesSpawned(pub bool);

fn should_spawn_amenities(
    buildings_spawned: Res<BuildingsSpawned>,
    amenities_spawned: Res<StreetAmenitiesSpawned>,
) -> bool {
    buildings_spawned.0 && !amenities_spawned.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AmenityType {
    PhoneBooth,
    ChargingStation,
    BikeRack,
    CafeSeating,
}

/// Marker component for street amenities.
#[derive(Component)]
pub struct StreetAmenity {
    pub amenity_type: AmenityType,
}

#[derive(Resource)]
pub struct StreetAmenitiesConfig {
    pub seed: u64,
    pub max_phone_booths: usize,
    pub max_charging_stations: usize,
    pub max_bike_racks: usize,
    pub max_cafe_seating: usize,
    pub min_spacing: f32,
}

impl Default for StreetAmenitiesConfig {
    fn default() -> Self {
        Self {
            seed: 22222,
            max_phone_booths: 15,
            max_charging_stations: 12,
            max_bike_racks: 20,
            max_cafe_seating: 25,
            min_spacing: 20.0,
        }
    }
}

fn spawn_street_amenities(
    mut commands: Commands,
    config: Res<StreetAmenitiesConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<StreetAmenitiesSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Collect building positions by type
    let commercial_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| b.building_type == BuildingArchetype::Commercial)
        .map(|(_, t)| t.translation)
        .collect();

    let all_positions: Vec<Vec3> = buildings.iter().map(|(_, t)| t.translation).collect();

    // Common materials
    let metal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.45, 0.48),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.75, 0.8, 0.4),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });

    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Spawn phone booths
    let mut phone_count = 0;
    for &pos in all_positions.iter() {
        if phone_count >= config.max_phone_booths {
            break;
        }
        if rng.gen::<f32>() > 0.15 {
            continue;
        }

        let offset = Vec3::new(rng.gen_range(-6.0..6.0), 0.0, rng.gen_range(-6.0..6.0));
        let booth_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        if placed_positions.iter().any(|p| p.distance(booth_pos) < config.min_spacing) {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen_range(0..4) as f32 * PI / 2.0);
        spawn_phone_booth(&mut commands, &mut meshes, &mut materials, &metal_material, &glass_material, booth_pos, rotation);

        placed_positions.push(booth_pos);
        phone_count += 1;
    }

    // Spawn charging stations
    let mut charging_count = 0;
    for &pos in commercial_positions.iter() {
        if charging_count >= config.max_charging_stations {
            break;
        }
        if rng.gen::<f32>() > 0.2 {
            continue;
        }

        let offset = Vec3::new(rng.gen_range(-8.0..8.0), 0.0, rng.gen_range(-8.0..8.0));
        let station_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        if placed_positions.iter().any(|p| p.distance(station_pos) < config.min_spacing) {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);
        spawn_charging_station(&mut commands, &mut meshes, &mut materials, station_pos, rotation);

        placed_positions.push(station_pos);
        charging_count += 1;
    }

    // Spawn bike racks
    let mut bike_count = 0;
    for &pos in all_positions.iter() {
        if bike_count >= config.max_bike_racks {
            break;
        }
        if rng.gen::<f32>() > 0.2 {
            continue;
        }

        let offset = Vec3::new(rng.gen_range(-8.0..8.0), 0.0, rng.gen_range(-8.0..8.0));
        let rack_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        if placed_positions.iter().any(|p| p.distance(rack_pos) < config.min_spacing) {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);
        let has_bikes = rng.gen::<f32>() < 0.7;
        spawn_bike_rack(&mut commands, &mut meshes, &mut materials, &metal_material, rack_pos, rotation, has_bikes, &mut rng);

        placed_positions.push(rack_pos);
        bike_count += 1;
    }

    // Spawn café seating (only near commercial)
    let mut cafe_count = 0;
    for &pos in commercial_positions.iter() {
        if cafe_count >= config.max_cafe_seating {
            break;
        }
        if rng.gen::<f32>() > 0.3 {
            continue;
        }

        let offset = Vec3::new(rng.gen_range(-5.0..5.0), 0.0, rng.gen_range(-5.0..5.0));
        let cafe_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        if placed_positions.iter().any(|p| p.distance(cafe_pos) < config.min_spacing * 0.8) {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);
        spawn_cafe_seating(&mut commands, &mut meshes, &mut materials, &metal_material, &wood_material, cafe_pos, rotation, &mut rng);

        placed_positions.push(cafe_pos);
        cafe_count += 1;
    }

    info!(
        "Spawned {} phone booths, {} charging stations, {} bike racks, {} café seating areas",
        phone_count, charging_count, bike_count, cafe_count
    );
}

fn spawn_phone_booth(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    metal_material: &Handle<StandardMaterial>,
    glass_material: &Handle<StandardMaterial>,
    position: Vec3,
    rotation: Quat,
) {
    commands
        .spawn((
            Transform::from_translation(position).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            StreetAmenity { amenity_type: AmenityType::PhoneBooth },
        ))
        .with_children(|parent| {
            // Frame
            let frame_mesh = meshes.add(Cuboid::new(1.0, 2.2, 1.0));
            let frame_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.2, 0.2),
                perceptual_roughness: 0.5,
                ..default()
            });
            parent.spawn((
                Mesh3d(frame_mesh),
                MeshMaterial3d(frame_material),
                Transform::from_xyz(0.0, 1.1, 0.0),
            ));

            // Glass panels (3 sides)
            let glass_mesh = meshes.add(Cuboid::new(0.9, 1.8, 0.05));
            for (x, z, ry) in [(0.0, 0.5, 0.0), (0.5, 0.0, PI / 2.0), (-0.5, 0.0, PI / 2.0)] {
                parent.spawn((
                    Mesh3d(glass_mesh.clone()),
                    MeshMaterial3d(glass_material.clone()),
                    Transform::from_xyz(x, 1.1, z)
                        .with_rotation(Quat::from_rotation_y(ry)),
                ));
            }

            // Phone unit inside
            let phone_mesh = meshes.add(Cuboid::new(0.3, 0.4, 0.1));
            parent.spawn((
                Mesh3d(phone_mesh),
                MeshMaterial3d(metal_material.clone()),
                Transform::from_xyz(0.0, 1.3, -0.35),
            ));

            // Roof
            let roof_mesh = meshes.add(Cuboid::new(1.1, 0.1, 1.1));
            parent.spawn((
                Mesh3d(roof_mesh),
                MeshMaterial3d(metal_material.clone()),
                Transform::from_xyz(0.0, 2.25, 0.0),
            ));
        });
}

fn spawn_charging_station(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    rotation: Quat,
) {
    let station_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.3),
        perceptual_roughness: 0.5,
        metallic: 0.3,
        ..default()
    });

    let screen_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.12),
        emissive: LinearRgba::new(0.0, 0.2, 0.1, 1.0),
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(position).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            StreetAmenity { amenity_type: AmenityType::ChargingStation },
        ))
        .with_children(|parent| {
            // Main post
            let post_mesh = meshes.add(Cuboid::new(0.3, 1.5, 0.3));
            parent.spawn((
                Mesh3d(post_mesh),
                MeshMaterial3d(station_material.clone()),
                Transform::from_xyz(0.0, 0.75, 0.0),
            ));

            // Screen
            let screen_mesh = meshes.add(Cuboid::new(0.25, 0.2, 0.05));
            parent.spawn((
                Mesh3d(screen_mesh),
                MeshMaterial3d(screen_material),
                Transform::from_xyz(0.0, 1.3, 0.18),
            ));

            // Charging ports
            let port_mesh = meshes.add(Cuboid::new(0.08, 0.05, 0.02));
            let port_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                ..default()
            });
            for y in [0.8, 0.9, 1.0] {
                parent.spawn((
                    Mesh3d(port_mesh.clone()),
                    MeshMaterial3d(port_material.clone()),
                    Transform::from_xyz(0.0, y, 0.18),
                ));
            }
        });
}

fn spawn_bike_rack(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    metal_material: &Handle<StandardMaterial>,
    position: Vec3,
    rotation: Quat,
    has_bikes: bool,
    rng: &mut StdRng,
) {
    commands
        .spawn((
            Transform::from_translation(position).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            StreetAmenity { amenity_type: AmenityType::BikeRack },
        ))
        .with_children(|parent| {
            // Base bar
            let base_mesh = meshes.add(Cuboid::new(2.0, 0.05, 0.05));
            parent.spawn((
                Mesh3d(base_mesh),
                MeshMaterial3d(metal_material.clone()),
                Transform::from_xyz(0.0, 0.025, 0.0),
            ));

            // Upright loops
            let loop_mesh = meshes.add(Torus::new(0.02, 0.25));
            for x in [-0.6, 0.0, 0.6] {
                parent.spawn((
                    Mesh3d(loop_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_xyz(x, 0.5, 0.0)
                        .with_rotation(Quat::from_rotation_x(PI / 2.0)),
                ));

                // Vertical post for loop
                let post_mesh = meshes.add(Cylinder::new(0.02, 0.5));
                parent.spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_xyz(x, 0.25, 0.0),
                ));
            }

            // Add bikes if present
            if has_bikes {
                let bike_colors = [
                    Color::srgb(0.2, 0.2, 0.8),
                    Color::srgb(0.8, 0.2, 0.2),
                    Color::srgb(0.2, 0.6, 0.2),
                    Color::srgb(0.1, 0.1, 0.1),
                ];

                for (i, x) in [-0.6, 0.0, 0.6].iter().enumerate() {
                    if rng.gen::<f32>() < 0.6 {
                        let bike_material = materials.add(StandardMaterial {
                            base_color: bike_colors[rng.gen_range(0..bike_colors.len())],
                            metallic: 0.5,
                            perceptual_roughness: 0.4,
                            ..default()
                        });

                        // Simplified bike (frame triangle)
                        let frame_mesh = meshes.add(Cuboid::new(0.6, 0.02, 0.02));
                        parent.spawn((
                            Mesh3d(frame_mesh.clone()),
                            MeshMaterial3d(bike_material.clone()),
                            Transform::from_xyz(*x, 0.4, 0.15)
                                .with_rotation(Quat::from_rotation_z(0.3)),
                        ));

                        // Wheels
                        let wheel_mesh = meshes.add(Torus::new(0.01, 0.15));
                        let wheel_material = materials.add(StandardMaterial {
                            base_color: Color::srgb(0.1, 0.1, 0.1),
                            ..default()
                        });
                        for wx in [-0.25, 0.25] {
                            parent.spawn((
                                Mesh3d(wheel_mesh.clone()),
                                MeshMaterial3d(wheel_material.clone()),
                                Transform::from_xyz(*x + wx, 0.15, 0.15)
                                    .with_rotation(Quat::from_rotation_y(PI / 2.0)),
                            ));
                        }
                    }
                }
            }
        });
}

fn spawn_cafe_seating(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    metal_material: &Handle<StandardMaterial>,
    wood_material: &Handle<StandardMaterial>,
    position: Vec3,
    rotation: Quat,
    rng: &mut StdRng,
) {
    // Umbrella colors
    let umbrella_colors = [
        Color::srgb(0.8, 0.2, 0.2),
        Color::srgb(0.2, 0.5, 0.7),
        Color::srgb(0.2, 0.6, 0.3),
        Color::srgb(0.9, 0.7, 0.2),
    ];

    let umbrella_material = materials.add(StandardMaterial {
        base_color: umbrella_colors[rng.gen_range(0..umbrella_colors.len())],
        perceptual_roughness: 0.8,
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(position).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            StreetAmenity { amenity_type: AmenityType::CafeSeating },
        ))
        .with_children(|parent| {
            // Table
            let table_top_mesh = meshes.add(Cylinder::new(0.4, 0.03));
            parent.spawn((
                Mesh3d(table_top_mesh),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(0.0, 0.75, 0.0),
            ));

            // Table leg
            let table_leg_mesh = meshes.add(Cylinder::new(0.04, 0.75));
            parent.spawn((
                Mesh3d(table_leg_mesh),
                MeshMaterial3d(metal_material.clone()),
                Transform::from_xyz(0.0, 0.375, 0.0),
            ));

            // Chairs around table
            let chair_seat_mesh = meshes.add(Cuboid::new(0.35, 0.03, 0.35));
            let chair_back_mesh = meshes.add(Cuboid::new(0.35, 0.4, 0.03));
            let chair_leg_mesh = meshes.add(Cylinder::new(0.015, 0.45));

            let num_chairs = rng.gen_range(2..=4);
            for i in 0..num_chairs {
                let angle = (i as f32 / num_chairs as f32) * PI * 2.0;
                let chair_dist = 0.7;
                let cx = angle.cos() * chair_dist;
                let cz = angle.sin() * chair_dist;
                let chair_rotation = Quat::from_rotation_y(-angle + PI);

                // Chair seat
                parent.spawn((
                    Mesh3d(chair_seat_mesh.clone()),
                    MeshMaterial3d(wood_material.clone()),
                    Transform::from_xyz(cx, 0.45, cz).with_rotation(chair_rotation),
                ));

                // Chair back
                parent.spawn((
                    Mesh3d(chair_back_mesh.clone()),
                    MeshMaterial3d(wood_material.clone()),
                    Transform::from_xyz(
                        cx - angle.cos() * 0.16,
                        0.65,
                        cz - angle.sin() * 0.16,
                    )
                    .with_rotation(chair_rotation),
                ));

                // Chair legs
                for (lx, lz) in [(-0.12, -0.12), (0.12, -0.12), (-0.12, 0.12), (0.12, 0.12)] {
                    let leg_x = cx + (lx * angle.cos() - lz * angle.sin());
                    let leg_z = cz + (lx * angle.sin() + lz * angle.cos());
                    parent.spawn((
                        Mesh3d(chair_leg_mesh.clone()),
                        MeshMaterial3d(metal_material.clone()),
                        Transform::from_xyz(leg_x, 0.225, leg_z),
                    ));
                }
            }

            // Umbrella
            let umbrella_mesh = meshes.add(Cone::new(1.0, 0.3));
            parent.spawn((
                Mesh3d(umbrella_mesh),
                MeshMaterial3d(umbrella_material),
                Transform::from_xyz(0.0, 2.2, 0.0)
                    .with_rotation(Quat::from_rotation_x(PI)),
            ));

            // Umbrella pole
            let pole_mesh = meshes.add(Cylinder::new(0.025, 1.5));
            parent.spawn((
                Mesh3d(pole_mesh),
                MeshMaterial3d(metal_material.clone()),
                Transform::from_xyz(0.0, 1.5, 0.0),
            ));
        });
}

//! Nature integration: planters, flower beds, green roofs, bird flocks.
//!
//! Adds natural elements to soften the urban environment.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct NatureDetailsPlugin;

impl Plugin for NatureDetailsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NatureDetailsConfig>()
            .init_resource::<NatureDetailsSpawned>()
            .add_systems(Update, spawn_nature_details.run_if(should_spawn_nature))
            .add_systems(Update, update_bird_flocks);
    }
}

#[derive(Resource, Default)]
pub struct NatureDetailsSpawned(pub bool);

fn should_spawn_nature(
    buildings_spawned: Res<BuildingsSpawned>,
    nature_spawned: Res<NatureDetailsSpawned>,
) -> bool {
    buildings_spawned.0 && !nature_spawned.0
}

/// Marker for planters.
#[derive(Component)]
pub struct Planter;

/// Marker for green roof.
#[derive(Component)]
pub struct GreenRoof;

/// Bird flock component with movement state.
#[derive(Component)]
pub struct BirdFlock {
    pub center: Vec3,
    pub radius: f32,
    pub phase: f32,
    pub speed: f32,
    pub height: f32,
}

#[derive(Resource)]
pub struct NatureDetailsConfig {
    pub seed: u64,
    pub max_planters: usize,
    pub green_roof_probability: f32,
    pub bird_flock_count: usize,
    pub min_spacing: f32,
}

impl Default for NatureDetailsConfig {
    fn default() -> Self {
        Self {
            seed: 55555,
            max_planters: 40,
            green_roof_probability: 0.15,
            bird_flock_count: 8,
            min_spacing: 15.0,
        }
    }
}

// Flower colors
const FLOWER_COLORS: &[(f32, f32, f32)] = &[
    (0.9, 0.3, 0.4),   // Pink
    (0.95, 0.9, 0.3),  // Yellow
    (0.9, 0.5, 0.2),   // Orange
    (0.7, 0.3, 0.7),   // Purple
    (0.3, 0.5, 0.8),   // Blue
    (0.95, 0.95, 0.95), // White
];

fn spawn_nature_details(
    mut commands: Commands,
    config: Res<NatureDetailsConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<NatureDetailsSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Materials
    let concrete_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.52),
        perceptual_roughness: 0.9,
        ..default()
    });

    let soil_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    });

    let grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.45, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });

    let bird_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.18),
        perceptual_roughness: 0.7,
        ..default()
    });

    // Collect building positions
    let commercial_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| b.building_type == BuildingArchetype::Commercial)
        .map(|(_, t)| t.translation)
        .collect();

    let all_buildings: Vec<(&Building, Vec3)> = buildings
        .iter()
        .map(|(b, t)| (b, t.translation))
        .collect();

    // Spawn planters near commercial buildings
    let mut planter_count = 0;
    for &pos in commercial_positions.iter() {
        if planter_count >= config.max_planters {
            break;
        }

        for _ in 0..3 {
            if rng.gen::<f32>() > 0.4 {
                continue;
            }

            let offset = Vec3::new(
                rng.gen_range(-6.0..6.0),
                0.0,
                rng.gen_range(-6.0..6.0),
            );
            let planter_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

            if placed_positions.iter().any(|p| p.distance(planter_pos) < config.min_spacing) {
                continue;
            }

            let planter_type = rng.gen_range(0..3);
            spawn_planter(
                &mut commands,
                &mut meshes,
                &mut materials,
                &concrete_material,
                &soil_material,
                planter_pos,
                planter_type,
                &mut rng,
            );

            placed_positions.push(planter_pos);
            planter_count += 1;

            if planter_count >= config.max_planters {
                break;
            }
        }
    }

    // Spawn green roofs on some buildings
    let mut green_roof_count = 0;
    for (building, pos) in all_buildings.iter() {
        if rng.gen::<f32>() > config.green_roof_probability {
            continue;
        }

        // Prefer residential and some commercial
        if !matches!(
            building.building_type,
            BuildingArchetype::Residential | BuildingArchetype::Commercial
        ) {
            continue;
        }

        // Estimate building height (rough)
        let building_height = rng.gen_range(8.0..25.0);
        let roof_pos = Vec3::new(pos.x, building_height, pos.z);

        spawn_green_roof(
            &mut commands,
            &mut meshes,
            &grass_material,
            roof_pos,
            rng.gen_range(6.0..12.0),
            rng.gen_range(6.0..12.0),
        );

        green_roof_count += 1;
    }

    // Spawn bird flocks
    let city_center = Vec3::ZERO;
    let city_radius = 200.0;

    for i in 0..config.bird_flock_count {
        let angle = (i as f32 / config.bird_flock_count as f32) * PI * 2.0;
        let dist = rng.gen_range(50.0..city_radius);
        let center = Vec3::new(
            city_center.x + angle.cos() * dist,
            rng.gen_range(30.0..60.0),
            city_center.z + angle.sin() * dist,
        );

        spawn_bird_flock(
            &mut commands,
            &mut meshes,
            &bird_material,
            center,
            rng.gen_range(5.0..15.0),
            rng.gen_range(0.5..1.5),
            &mut rng,
        );
    }

    info!(
        "Spawned {} planters, {} green roofs, {} bird flocks",
        planter_count, green_roof_count, config.bird_flock_count
    );
}

fn spawn_planter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    concrete_material: &Handle<StandardMaterial>,
    soil_material: &Handle<StandardMaterial>,
    position: Vec3,
    planter_type: u32,
    rng: &mut StdRng,
) {
    commands
        .spawn((
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Planter,
        ))
        .with_children(|parent| {
            match planter_type {
                0 => {
                    // Rectangular planter
                    let box_mesh = meshes.add(Cuboid::new(1.5, 0.5, 0.6));
                    parent.spawn((
                        Mesh3d(box_mesh),
                        MeshMaterial3d(concrete_material.clone()),
                        Transform::from_xyz(0.0, 0.25, 0.0),
                    ));

                    let soil_mesh = meshes.add(Cuboid::new(1.4, 0.1, 0.5));
                    parent.spawn((
                        Mesh3d(soil_mesh),
                        MeshMaterial3d(soil_material.clone()),
                        Transform::from_xyz(0.0, 0.45, 0.0),
                    ));

                    // Flowers/plants
                    spawn_flowers(parent, meshes, materials, 1.3, 0.4, 0.55, rng);
                }
                1 => {
                    // Round planter
                    let pot_mesh = meshes.add(Cylinder::new(0.4, 0.5));
                    parent.spawn((
                        Mesh3d(pot_mesh),
                        MeshMaterial3d(concrete_material.clone()),
                        Transform::from_xyz(0.0, 0.25, 0.0),
                    ));

                    let soil_mesh = meshes.add(Cylinder::new(0.35, 0.1));
                    parent.spawn((
                        Mesh3d(soil_mesh),
                        MeshMaterial3d(soil_material.clone()),
                        Transform::from_xyz(0.0, 0.45, 0.0),
                    ));

                    // Central plant
                    let plant_mesh = meshes.add(Sphere::new(0.25));
                    let plant_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.2, 0.5, 0.25),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    parent.spawn((
                        Mesh3d(plant_mesh),
                        MeshMaterial3d(plant_material),
                        Transform::from_xyz(0.0, 0.7, 0.0),
                    ));
                }
                _ => {
                    // Large raised bed
                    let bed_mesh = meshes.add(Cuboid::new(2.0, 0.4, 1.0));
                    parent.spawn((
                        Mesh3d(bed_mesh),
                        MeshMaterial3d(concrete_material.clone()),
                        Transform::from_xyz(0.0, 0.2, 0.0),
                    ));

                    let soil_mesh = meshes.add(Cuboid::new(1.9, 0.15, 0.9));
                    parent.spawn((
                        Mesh3d(soil_mesh),
                        MeshMaterial3d(soil_material.clone()),
                        Transform::from_xyz(0.0, 0.35, 0.0),
                    ));

                    spawn_flowers(parent, meshes, materials, 1.8, 0.8, 0.45, rng);

                    // Small shrub
                    let shrub_mesh = meshes.add(Sphere::new(0.3));
                    let shrub_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.15, 0.4, 0.2),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    parent.spawn((
                        Mesh3d(shrub_mesh),
                        MeshMaterial3d(shrub_material),
                        Transform::from_xyz(0.0, 0.65, 0.0),
                    ));
                }
            }
        });
}

fn spawn_flowers(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    width: f32,
    depth: f32,
    base_y: f32,
    rng: &mut StdRng,
) {
    let flower_mesh = meshes.add(Sphere::new(0.06));
    let stem_mesh = meshes.add(Cylinder::new(0.015, 0.15));
    let stem_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    let flower_count = rng.gen_range(6..12);
    for _ in 0..flower_count {
        let x = rng.gen_range(-width / 2.0..width / 2.0);
        let z = rng.gen_range(-depth / 2.0..depth / 2.0);
        let height = rng.gen_range(0.1..0.2);

        // Stem
        parent.spawn((
            Mesh3d(stem_mesh.clone()),
            MeshMaterial3d(stem_material.clone()),
            Transform::from_xyz(x, base_y + height / 2.0, z),
        ));

        // Flower head
        let (r, g, b) = FLOWER_COLORS[rng.gen_range(0..FLOWER_COLORS.len())];
        let flower_material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.8,
            ..default()
        });
        parent.spawn((
            Mesh3d(flower_mesh.clone()),
            MeshMaterial3d(flower_material),
            Transform::from_xyz(x, base_y + height + 0.05, z),
        ));
    }
}

fn spawn_green_roof(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grass_material: &Handle<StandardMaterial>,
    position: Vec3,
    width: f32,
    depth: f32,
) {
    let roof_mesh = meshes.add(Cuboid::new(width, 0.15, depth));

    commands.spawn((
        Mesh3d(roof_mesh),
        MeshMaterial3d(grass_material.clone()),
        Transform::from_translation(position),
        GreenRoof,
    ));
}

fn spawn_bird_flock(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    bird_material: &Handle<StandardMaterial>,
    center: Vec3,
    radius: f32,
    speed: f32,
    rng: &mut StdRng,
) {
    let bird_mesh = meshes.add(Cuboid::new(0.15, 0.05, 0.3));
    let wing_mesh = meshes.add(Cuboid::new(0.25, 0.02, 0.1));

    let bird_count = rng.gen_range(5..15);

    commands
        .spawn((
            Transform::from_translation(center),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            BirdFlock {
                center,
                radius,
                phase: rng.gen::<f32>() * PI * 2.0,
                speed,
                height: center.y,
            },
        ))
        .with_children(|parent| {
            for i in 0..bird_count {
                let angle = (i as f32 / bird_count as f32) * PI * 2.0;
                let dist = rng.gen_range(0.5..1.0) * radius * 0.3;
                let x = angle.cos() * dist;
                let y = rng.gen_range(-1.0..1.0);
                let z = angle.sin() * dist;

                // Bird body
                parent.spawn((
                    Mesh3d(bird_mesh.clone()),
                    MeshMaterial3d(bird_material.clone()),
                    Transform::from_xyz(x, y, z),
                ));

                // Wings
                parent.spawn((
                    Mesh3d(wing_mesh.clone()),
                    MeshMaterial3d(bird_material.clone()),
                    Transform::from_xyz(x - 0.15, y + 0.03, z),
                ));
                parent.spawn((
                    Mesh3d(wing_mesh.clone()),
                    MeshMaterial3d(bird_material.clone()),
                    Transform::from_xyz(x + 0.15, y + 0.03, z),
                ));
            }
        });
}

fn update_bird_flocks(time: Res<Time>, mut flocks: Query<(&BirdFlock, &mut Transform)>) {
    let t = time.elapsed_secs();

    for (flock, mut transform) in flocks.iter_mut() {
        let phase = flock.phase + t * flock.speed;
        let x = flock.center.x + phase.cos() * flock.radius;
        let z = flock.center.z + phase.sin() * flock.radius;
        let y = flock.height + (phase * 0.5).sin() * 5.0;

        transform.translation = Vec3::new(x, y, z);

        // Face direction of movement
        let dir = Vec2::new(-phase.sin(), phase.cos());
        let angle = dir.y.atan2(dir.x);
        transform.rotation = Quat::from_rotation_y(-angle + PI / 2.0);
    }
}

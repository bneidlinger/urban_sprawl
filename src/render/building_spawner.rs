//! Building spawner that generates varied buildings on city lots.

#![allow(dead_code)]

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::{
    BuildingArchetype, BuildingBlueprints, BuildingShape, FacadeStyle, PlannedStructure,
};
use crate::render::instancing::TerrainConfig;

pub struct BuildingSpawnerPlugin;

impl Plugin for BuildingSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingConfig>()
            .init_resource::<BuildingsSpawned>()
            .add_systems(Update, spawn_buildings.run_if(should_spawn_buildings));
    }
}

fn should_spawn_buildings(
    blueprints: Res<BuildingBlueprints>,
    spawned: Res<BuildingsSpawned>,
) -> bool {
    blueprints.generated && !spawned.0
}

#[derive(Resource, Default)]
pub struct BuildingsSpawned(pub bool);

#[derive(Component)]
pub struct Building {
    pub lot_index: usize,
    pub building_type: BuildingArchetype,
}

#[derive(Resource)]
pub struct BuildingConfig {
    pub seed: u64,
}

impl Default for BuildingConfig {
    fn default() -> Self {
        Self { seed: 42 }
    }
}

fn spawn_buildings(
    mut commands: Commands,
    blueprints: Res<BuildingBlueprints>,
    config: Res<BuildingConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BuildingsSpawned>,
) {
    info!("Spawning {} planned structures...", blueprints.plans.len());

    let terrain = TerrainSampler::new(&terrain_config);
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Facade palettes chosen to emphasize zone/type variety.
    let brick_colors = [
        Color::srgb(0.74, 0.48, 0.38),
        Color::srgb(0.68, 0.42, 0.32),
        Color::srgb(0.8, 0.54, 0.42),
    ];
    let concrete_colors = [
        Color::srgb(0.65, 0.65, 0.65),
        Color::srgb(0.72, 0.72, 0.72),
        Color::srgb(0.58, 0.6, 0.62),
    ];
    let glass_colors = [
        Color::srgb(0.4, 0.6, 0.75),
        Color::srgb(0.35, 0.55, 0.7),
        Color::srgb(0.45, 0.65, 0.8),
    ];
    let metal_colors = [
        Color::srgb(0.55, 0.55, 0.58),
        Color::srgb(0.5, 0.5, 0.52),
        Color::srgb(0.48, 0.5, 0.55),
    ];
    let painted_colors = [
        Color::srgb(0.85, 0.78, 0.62),
        Color::srgb(0.92, 0.86, 0.72),
        Color::srgb(0.76, 0.82, 0.72),
    ];

    // Park materials
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.55, 0.25),
        perceptual_roughness: 0.95,
        ..default()
    });
    let trunk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.28, 0.18),
        perceptual_roughness: 0.9,
        ..default()
    });
    let foliage_materials: Vec<Handle<StandardMaterial>> = [
        Color::srgb(0.2, 0.45, 0.15),
        Color::srgb(0.25, 0.5, 0.2),
        Color::srgb(0.18, 0.42, 0.12),
    ]
    .iter()
    .map(|&c| {
        materials.add(StandardMaterial {
            base_color: c,
            perceptual_roughness: 0.85,
            ..default()
        })
    })
    .collect();

    let trunk_mesh = meshes.add(Cylinder::new(0.3, 1.0));
    let foliage_mesh = meshes.add(Sphere::new(1.0));

    for plan in &blueprints.plans {
        match plan {
            PlannedStructure::Park(park) => {
                let terrain_height = terrain.sample(park.center.x, park.center.y);
                spawn_park(
                    &mut commands,
                    &mut meshes,
                    &grass_material,
                    &trunk_material,
                    &trunk_mesh,
                    &foliage_materials,
                    &foliage_mesh,
                    park.center,
                    park.size,
                    terrain_height,
                    &mut rng,
                );
            }
            PlannedStructure::Building(plan) => {
                let material = match plan.facade {
                    FacadeStyle::Brick => materials.add(StandardMaterial {
                        base_color: brick_colors[rng.gen_range(0..brick_colors.len())],
                        perceptual_roughness: 0.9,
                        metallic: 0.0,
                        ..default()
                    }),
                    FacadeStyle::Concrete => materials.add(StandardMaterial {
                        base_color: concrete_colors[rng.gen_range(0..concrete_colors.len())],
                        perceptual_roughness: 0.85,
                        metallic: 0.0,
                        ..default()
                    }),
                    FacadeStyle::Glass => materials.add(StandardMaterial {
                        base_color: glass_colors[rng.gen_range(0..glass_colors.len())],
                        perceptual_roughness: 0.2,
                        metallic: 0.1,
                        ..default()
                    }),
                    FacadeStyle::Metal => materials.add(StandardMaterial {
                        base_color: metal_colors[rng.gen_range(0..metal_colors.len())],
                        perceptual_roughness: 0.35,
                        metallic: 0.6,
                        ..default()
                    }),
                    FacadeStyle::Painted => materials.add(StandardMaterial {
                        base_color: painted_colors[rng.gen_range(0..painted_colors.len())],
                        perceptual_roughness: 0.75,
                        metallic: 0.0,
                        ..default()
                    }),
                };

                // Sample terrain height at building center
                let terrain_height = terrain.sample(plan.center.x, plan.center.y);

                // Spawn building based on shape
                match plan.shape {
                    BuildingShape::Box => {
                        spawn_box_building(
                            &mut commands,
                            &mut meshes,
                            material,
                            plan.center,
                            plan.footprint,
                            plan.height,
                            terrain_height,
                            plan.lot_index,
                            plan.building_type,
                        );
                    }
                    BuildingShape::LShape => {
                        spawn_l_building(
                            &mut commands,
                            &mut meshes,
                            material,
                            plan.center,
                            plan.footprint,
                            plan.height,
                            terrain_height,
                            &mut rng,
                            plan.lot_index,
                            plan.building_type,
                        );
                    }
                    BuildingShape::TowerOnBase => {
                        spawn_tower_building(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            material,
                            plan.center,
                            plan.footprint,
                            plan.height,
                            terrain_height,
                            &mut rng,
                            plan.lot_index,
                            plan.building_type,
                        );
                    }
                    BuildingShape::Stepped => {
                        spawn_stepped_building(
                            &mut commands,
                            &mut meshes,
                            material,
                            plan.center,
                            plan.footprint,
                            plan.height,
                            terrain_height,
                            &mut rng,
                            plan.lot_index,
                            plan.building_type,
                        );
                    }
                }
            }
        }
    }

    spawned.0 = true;
    info!("Buildings spawned");
}

fn spawn_box_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    let mesh = meshes.add(Cuboid::new(size.x, height, size.y));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(center.x, terrain_height + height / 2.0, center.y),
        Building {
            lot_index,
            building_type,
        },
    ));
}

fn spawn_l_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    rng: &mut StdRng,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    // L-shape: two overlapping rectangles
    let wing_ratio = 0.4 + rng.gen::<f32>() * 0.2; // 40-60% of size

    // Main wing
    let main_size = Vec2::new(size.x, size.y * wing_ratio);
    let main_mesh = meshes.add(Cuboid::new(main_size.x, height, main_size.y));
    let main_offset = Vec2::new(0.0, (size.y - main_size.y) / 2.0);

    commands.spawn((
        Mesh3d(main_mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(
            center.x + main_offset.x,
            terrain_height + height / 2.0,
            center.y + main_offset.y,
        ),
        Building {
            lot_index,
            building_type,
        },
    ));

    // Side wing
    let side_size = Vec2::new(size.x * wing_ratio, size.y - main_size.y + 1.0);
    let side_mesh = meshes.add(Cuboid::new(side_size.x, height, side_size.y));

    // Rotate L randomly
    let side_x = if rng.gen::<bool>() {
        center.x - (size.x - side_size.x) / 2.0
    } else {
        center.x + (size.x - side_size.x) / 2.0
    };
    let side_z = center.y - main_size.y / 2.0;

    commands.spawn((
        Mesh3d(side_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(side_x, terrain_height + height / 2.0, side_z),
        Building {
            lot_index,
            building_type,
        },
    ));
}

fn spawn_tower_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    rng: &mut StdRng,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    // Base podium
    let base_height = 4.0 + rng.gen::<f32>() * 4.0;
    let base_mesh = meshes.add(Cuboid::new(size.x, base_height, size.y));

    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(center.x, terrain_height + base_height / 2.0, center.y),
        Building {
            lot_index,
            building_type,
        },
    ));

    // Tower on top
    let tower_ratio = 0.5 + rng.gen::<f32>() * 0.2;
    let tower_size = size * tower_ratio;
    let tower_height = height - base_height;

    if tower_height > 2.0 {
        let tower_mesh = meshes.add(Cuboid::new(tower_size.x, tower_height, tower_size.y));

        // Slightly different shade for tower
        let tower_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.55, 0.65),
            perceptual_roughness: 0.2,
            metallic: 0.3,
            ..default()
        });

        commands.spawn((
            Mesh3d(tower_mesh),
            MeshMaterial3d(tower_material),
            Transform::from_xyz(
                center.x,
                terrain_height + base_height + tower_height / 2.0,
                center.y,
            ),
            Building {
                lot_index,
                building_type,
            },
        ));
    }
}

fn spawn_stepped_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    rng: &mut StdRng,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    let num_steps = 2 + rng.gen_range(0..2);
    let step_height = height / num_steps as f32;

    for i in 0..num_steps {
        let shrink = 1.0 - (i as f32 * 0.15);
        let step_size = size * shrink;
        let y = terrain_height + step_height * i as f32 + step_height / 2.0;

        let step_mesh = meshes.add(Cuboid::new(step_size.x, step_height, step_size.y));

        commands.spawn((
            Mesh3d(step_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(center.x, y, center.y),
            Building {
                lot_index,
                building_type,
            },
        ));
    }
}

/// Marker for park entities.
#[derive(Component)]
pub struct Park;

/// Marker for tree entities.
#[derive(Component)]
pub struct Tree;

fn spawn_park(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    grass_material: &Handle<StandardMaterial>,
    trunk_material: &Handle<StandardMaterial>,
    trunk_mesh: &Handle<Mesh>,
    foliage_materials: &[Handle<StandardMaterial>],
    foliage_mesh: &Handle<Mesh>,
    center: Vec2,
    size: Vec2,
    terrain_height: f32,
    rng: &mut StdRng,
) {
    // Grass ground
    let grass_mesh = meshes.add(Cuboid::new(size.x - 0.5, 0.15, size.y - 0.5));
    commands.spawn((
        Mesh3d(grass_mesh),
        MeshMaterial3d(grass_material.clone()),
        Transform::from_xyz(center.x, terrain_height + 0.075, center.y),
        Park,
    ));

    // Trees
    let num_trees = rng.gen_range(2..=5);
    for _ in 0..num_trees {
        let tree_x = center.x + rng.gen_range(-size.x / 3.0..size.x / 3.0);
        let tree_z = center.y + rng.gen_range(-size.y / 3.0..size.y / 3.0);

        // Realistic tree height: 6-12m
        let tree_height = 6.0 + rng.gen::<f32>() * 6.0;
        let foliage_size = 2.5 + rng.gen::<f32>() * 1.5;

        // Trunk
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_material.clone()),
            Transform::from_xyz(tree_x, terrain_height + tree_height / 2.0, tree_z)
                .with_scale(Vec3::new(1.0, tree_height, 1.0)),
            Tree,
        ));

        // Foliage
        let foliage_mat = foliage_materials[rng.gen_range(0..foliage_materials.len())].clone();
        commands.spawn((
            Mesh3d(foliage_mesh.clone()),
            MeshMaterial3d(foliage_mat),
            Transform::from_xyz(
                tree_x,
                terrain_height + tree_height + foliage_size * 0.3,
                tree_z,
            )
            .with_scale(Vec3::splat(foliage_size)),
            Tree,
        ));
    }
}

/// Helper struct for sampling terrain height.
struct TerrainSampler {
    perlin: Perlin,
    noise_scale: f32,
    height_scale: f32,
    octaves: u32,
}

impl TerrainSampler {
    fn new(config: &TerrainConfig) -> Self {
        Self {
            perlin: Perlin::new(config.seed),
            noise_scale: config.noise_scale,
            height_scale: config.height_scale,
            octaves: config.octaves,
        }
    }

    fn sample(&self, x: f32, z: f32) -> f32 {
        let mut height = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.noise_scale;
        let mut max_amplitude = 0.0;

        for _ in 0..self.octaves {
            let sample_x = x as f64 * frequency as f64;
            let sample_z = z as f64 * frequency as f64;
            height += self.perlin.get([sample_x, sample_z]) as f32 * amplitude;
            max_amplitude += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        (height / max_amplitude) * self.height_scale
    }
}

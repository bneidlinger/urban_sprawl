//! Building spawner that generates varied buildings on city lots.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::procgen::block_extractor::CityLots;
use crate::procgen::parcels::Lot;
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
    lots: Res<CityLots>,
    spawned: Res<BuildingsSpawned>,
) -> bool {
    !lots.lots.is_empty() && !spawned.0
}

#[derive(Resource, Default)]
pub struct BuildingsSpawned(pub bool);

#[derive(Component)]
pub struct Building {
    pub lot_index: usize,
    pub building_type: BuildingType,
}

#[derive(Clone, Copy, Debug)]
pub enum BuildingType {
    Residential,
    Commercial,
    Industrial,
}

#[derive(Clone, Copy, Debug)]
pub enum BuildingShape {
    Box,
    LShape,
    TowerOnBase,
    Stepped,
}

#[derive(Resource)]
pub struct BuildingConfig {
    pub min_height: f32,
    pub max_height_residential: f32,
    pub max_height_commercial: f32,
    pub max_height_industrial: f32,
    pub setback: f32,
    pub seed: u64,
}

impl Default for BuildingConfig {
    fn default() -> Self {
        Self {
            min_height: 6.0,              // 2-floor minimum (~3m per floor)
            max_height_residential: 18.0, // Up to 6 floors
            max_height_commercial: 60.0,  // Up to 20 floors downtown
            max_height_industrial: 15.0,  // Lower industrial buildings
            setback: 2.0,
            seed: 42,
        }
    }
}

fn spawn_buildings(
    mut commands: Commands,
    lots: Res<CityLots>,
    config: Res<BuildingConfig>,
    terrain_config: Res<TerrainConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BuildingsSpawned>,
) {
    info!("Spawning {} buildings...", lots.lots.len());

    let terrain = TerrainSampler::new(&terrain_config);
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Material palettes
    let residential_colors = [
        Color::srgb(0.85, 0.75, 0.65),
        Color::srgb(0.75, 0.70, 0.65),
        Color::srgb(0.80, 0.80, 0.75),
        Color::srgb(0.90, 0.85, 0.80),
        Color::srgb(0.88, 0.82, 0.72),
    ];

    let commercial_colors = [
        Color::srgb(0.4, 0.5, 0.6),
        Color::srgb(0.3, 0.4, 0.5),
        Color::srgb(0.5, 0.5, 0.55),
        Color::srgb(0.6, 0.65, 0.7),
        Color::srgb(0.45, 0.55, 0.65),
    ];

    let industrial_colors = [
        Color::srgb(0.5, 0.45, 0.4),
        Color::srgb(0.55, 0.5, 0.45),
        Color::srgb(0.6, 0.55, 0.5),
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
    .map(|&c| materials.add(StandardMaterial { base_color: c, perceptual_roughness: 0.85, ..default() }))
    .collect();

    let trunk_mesh = meshes.add(Cylinder::new(0.3, 1.0));
    let foliage_mesh = meshes.add(Sphere::new(1.0));

    for (lot_index, lot) in lots.lots.iter().enumerate() {
        if lot.vertices.len() < 3 {
            continue;
        }

        let centroid = lot_centroid(lot);
        let dist_from_center = centroid.length();

        // Calculate lot bounds early for park check
        let shrunk = shrink_polygon(&lot.vertices, config.setback);
        if shrunk.len() < 3 {
            continue;
        }

        let (min, max) = polygon_bounds(&shrunk);
        let size = max - min;

        if size.x < 4.0 || size.y < 4.0 {
            continue;
        }

        let center = (min + max) / 2.0;

        // 8% chance to be a park (more likely away from downtown)
        let park_chance = if dist_from_center > 150.0 { 0.10 } else { 0.04 };
        if rng.gen::<f32>() < park_chance && size.x > 6.0 && size.y > 6.0 {
            let park_terrain = terrain.sample(center.x, center.y);
            spawn_park(&mut commands, &mut meshes, &grass_material, &trunk_material, &trunk_mesh, &foliage_materials, &foliage_mesh, center, size, park_terrain, &mut rng);
            continue;
        }

        // Determine building type
        let building_type = if dist_from_center < 100.0 {
            BuildingType::Commercial
        } else if dist_from_center > 300.0 && rng.gen::<f32>() < 0.2 {
            BuildingType::Industrial
        } else {
            BuildingType::Residential
        };

        // Select building shape based on type and randomness
        let shape = select_building_shape(building_type, size, &mut rng);

        // Building heights
        let max_height = match building_type {
            BuildingType::Residential => config.max_height_residential,
            BuildingType::Commercial => config.max_height_commercial,
            BuildingType::Industrial => config.max_height_industrial,
        };

        let height = config.min_height + rng.gen::<f32>() * (max_height - config.min_height);

        // Select color
        let color = match building_type {
            BuildingType::Residential => residential_colors[rng.gen_range(0..residential_colors.len())],
            BuildingType::Commercial => commercial_colors[rng.gen_range(0..commercial_colors.len())],
            BuildingType::Industrial => industrial_colors[rng.gen_range(0..industrial_colors.len())],
        };

        let material = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: match building_type {
                BuildingType::Commercial => 0.3,
                _ => 0.8,
            },
            metallic: match building_type {
                BuildingType::Commercial => 0.2,
                _ => 0.0,
            },
            ..default()
        });

        // Sample terrain height at building center
        let terrain_height = terrain.sample(center.x, center.y);

        // Spawn building based on shape
        match shape {
            BuildingShape::Box => {
                spawn_box_building(&mut commands, &mut meshes, material, center, size, height, terrain_height, lot_index, building_type);
            }
            BuildingShape::LShape => {
                spawn_l_building(&mut commands, &mut meshes, material, center, size, height, terrain_height, &mut rng, lot_index, building_type);
            }
            BuildingShape::TowerOnBase => {
                spawn_tower_building(&mut commands, &mut meshes, &mut materials, material, center, size, height, terrain_height, &mut rng, lot_index, building_type);
            }
            BuildingShape::Stepped => {
                spawn_stepped_building(&mut commands, &mut meshes, material, center, size, height, terrain_height, &mut rng, lot_index, building_type);
            }
        }
    }

    spawned.0 = true;
    info!("Buildings spawned");
}

fn select_building_shape(building_type: BuildingType, size: Vec2, rng: &mut StdRng) -> BuildingShape {
    let roll = rng.gen::<f32>();

    match building_type {
        BuildingType::Commercial => {
            if roll < 0.3 {
                BuildingShape::TowerOnBase
            } else if roll < 0.5 {
                BuildingShape::Stepped
            } else if roll < 0.7 && size.x > 6.0 && size.y > 6.0 {
                BuildingShape::LShape
            } else {
                BuildingShape::Box
            }
        }
        BuildingType::Residential => {
            if roll < 0.25 && size.x > 6.0 && size.y > 6.0 {
                BuildingShape::LShape
            } else if roll < 0.35 {
                BuildingShape::Stepped
            } else {
                BuildingShape::Box
            }
        }
        BuildingType::Industrial => {
            if roll < 0.3 && size.x > 8.0 && size.y > 8.0 {
                BuildingShape::LShape
            } else {
                BuildingShape::Box
            }
        }
    }
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
    building_type: BuildingType,
) {
    let mesh = meshes.add(Cuboid::new(size.x, height, size.y));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(center.x, terrain_height + height / 2.0, center.y),
        Building { lot_index, building_type },
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
    building_type: BuildingType,
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
        Transform::from_xyz(center.x + main_offset.x, terrain_height + height / 2.0, center.y + main_offset.y),
        Building { lot_index, building_type },
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
        Building { lot_index, building_type },
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
    building_type: BuildingType,
) {
    // Base podium
    let base_height = 4.0 + rng.gen::<f32>() * 4.0;
    let base_mesh = meshes.add(Cuboid::new(size.x, base_height, size.y));

    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(center.x, terrain_height + base_height / 2.0, center.y),
        Building { lot_index, building_type },
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
            Transform::from_xyz(center.x, terrain_height + base_height + tower_height / 2.0, center.y),
            Building { lot_index, building_type },
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
    building_type: BuildingType,
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
            Building { lot_index, building_type },
        ));
    }
}

fn lot_centroid(lot: &Lot) -> Vec2 {
    if lot.vertices.is_empty() {
        return Vec2::ZERO;
    }
    lot.vertices.iter().copied().sum::<Vec2>() / lot.vertices.len() as f32
}

fn shrink_polygon(vertices: &[Vec2], distance: f32) -> Vec<Vec2> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let centroid: Vec2 = vertices.iter().copied().sum::<Vec2>() / vertices.len() as f32;
    vertices
        .iter()
        .map(|&v| {
            let dir = (v - centroid).normalize_or_zero();
            v - dir * distance
        })
        .collect()
}

fn polygon_bounds(vertices: &[Vec2]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for &v in vertices {
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
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
            Transform::from_xyz(tree_x, terrain_height + tree_height + foliage_size * 0.3, tree_z)
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

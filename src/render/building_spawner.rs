//! Building spawner that generates varied buildings on city lots.
//!
//! This module has been updated to use GPU instancing for efficient rendering
//! of thousands of buildings with minimal draw calls.
//!
//! Only runs in Procedural mode - Sandbox mode uses player-painted zones instead.

#![allow(dead_code)]

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::game_state::GameMode;
use crate::procgen::building_factory::{
    BuildingArchetype, BuildingBlueprints, BuildingShape, FacadeStyle, PlannedStructure,
};
use crate::render::building_instances::{
    BuildingInstanceBuffer, BuildingInstanceData, BuildingMaterialPalette, BuildingRef,
};
use crate::render::gpu_culling::GpuCullable;
use crate::render::instancing::TerrainConfig;
use crate::render::mesh_pools::{BuildingMeshPool, VegetationMeshPool};

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
    game_mode: Res<State<GameMode>>,
) -> bool {
    // Only spawn procedural buildings in Procedural mode - Sandbox uses player zones
    *game_mode.get() == GameMode::Procedural && blueprints.generated && !spawned.0
}

#[derive(Resource, Default)]
pub struct BuildingsSpawned(pub bool);

#[derive(Component)]
pub struct Building {
    pub lot_index: usize,
    pub building_type: BuildingArchetype,
    pub facade_style: FacadeStyle,
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
    mesh_pool: Res<BuildingMeshPool>,
    vegetation_pool: Res<VegetationMeshPool>,
    palette: Res<BuildingMaterialPalette>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut instance_buffer: ResMut<BuildingInstanceBuffer>,
    mut spawned: ResMut<BuildingsSpawned>,
) {
    info!(
        "Spawning {} planned structures with GPU instancing...",
        blueprints.plans.len()
    );

    let terrain = TerrainSampler::new(&terrain_config);
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Park materials (still use standard spawning for now)
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

    // Clear existing instance buffer
    instance_buffer.clear();

    // Count buildings and parks for logging
    let mut building_count = 0;
    let mut park_count = 0;

    for plan in &blueprints.plans {
        match plan {
            PlannedStructure::Park(park) => {
                let terrain_height = terrain.sample(park.center.x, park.center.y);
                spawn_park(
                    &mut commands,
                    &mut meshes,
                    &grass_material,
                    &trunk_material,
                    &vegetation_pool.trunk_mesh,
                    &foliage_materials,
                    &vegetation_pool.foliage_mesh,
                    park.center,
                    park.size,
                    terrain_height,
                    &mut rng,
                );
                park_count += 1;
            }
            PlannedStructure::Building(plan) => {
                // Get shared material from palette (enables GPU instancing!)
                let color_variant = rng.gen_range(0..3);
                let material = palette.get(plan.facade, color_variant)
                    .expect("Material palette not initialized");
                let color = BuildingMaterialPalette::get_color(plan.facade, color_variant);

                // Sample terrain height at building center
                let terrain_height = terrain.sample(plan.center.x, plan.center.y);

                // Create instance data and add to buffer
                let instance_index = spawn_building_instanced(
                    &mut commands,
                    &mut instance_buffer,
                    &mesh_pool,
                    &palette,
                    material.clone(),  // Use shared material!
                    plan.center,
                    plan.footprint,
                    plan.height,
                    terrain_height,
                    color,
                    plan.facade,
                    plan.building_type,
                    plan.shape,
                    plan.lot_index,
                    &mut rng,
                );

                // Spawn lightweight entity for ECS queries (no mesh, just reference)
                commands.spawn(BuildingRef {
                    lot_index: plan.lot_index,
                    instance_index,
                    archetype: plan.building_type,
                    facade: plan.facade,
                    shape: plan.shape,
                });

                building_count += 1;
            }
        }
    }

    // Update instance buffer stats
    instance_buffer.update_stats();
    instance_buffer.dirty = true;

    spawned.0 = true;
    info!(
        "Spawned {} buildings ({} instances) and {} parks",
        building_count,
        instance_buffer.stats.total_instances,
        park_count
    );
    info!(
        "Instance breakdown: {} box, {} L-shape, {} tower, {} stepped",
        instance_buffer.stats.box_count,
        instance_buffer.stats.l_shape_count,
        instance_buffer.stats.tower_count,
        instance_buffer.stats.stepped_count
    );
}

/// Spawn a building using instanced rendering.
/// Returns the instance index in the buffer.
fn spawn_building_instanced(
    commands: &mut Commands,
    instance_buffer: &mut BuildingInstanceBuffer,
    mesh_pool: &BuildingMeshPool,
    palette: &BuildingMaterialPalette,
    material: Handle<StandardMaterial>,  // Use pre-created shared material
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    color: Color,
    facade: FacadeStyle,
    archetype: BuildingArchetype,
    shape: BuildingShape,
    lot_index: usize,
    rng: &mut StdRng,
) -> usize {
    match shape {
        BuildingShape::Box => {
            // Simple box building - single instance
            let position = Vec3::new(center.x, terrain_height + height / 2.0, center.y);
            let scale = Vec3::new(size.x, height, size.y);

            let instance = BuildingInstanceData::new(
                position,
                scale,
                Quat::IDENTITY,
                color,
                facade,
                archetype,
                shape,
                lot_index,
            );

            // Spawn with shared material (enables Bevy's automatic GPU instancing)
            spawn_box_building_with_material(
                commands,
                mesh_pool.box_mesh.clone(),
                material.clone(),
                center,
                size,
                height,
                terrain_height,
                facade,
                lot_index,
                archetype,
            );

            instance_buffer.add(instance)
        }
        BuildingShape::LShape => {
            // L-shape uses two instances (main wing + side wing)
            let wing_ratio = 0.4 + rng.gen::<f32>() * 0.2;

            // Main wing
            let main_size = Vec2::new(size.x, size.y * wing_ratio);
            let main_offset = Vec2::new(0.0, (size.y - main_size.y) / 2.0);
            let main_pos = Vec3::new(
                center.x + main_offset.x,
                terrain_height + height / 2.0,
                center.y + main_offset.y,
            );
            let main_scale = Vec3::new(main_size.x, height, main_size.y);

            let main_instance = BuildingInstanceData::new(
                main_pos,
                main_scale,
                Quat::IDENTITY,
                color,
                facade,
                archetype,
                shape,
                lot_index,
            );
            let main_idx = instance_buffer.add(main_instance);

            // Side wing
            let side_size = Vec2::new(size.x * wing_ratio, size.y - main_size.y + 1.0);
            let side_x = if rng.gen::<bool>() {
                center.x - (size.x - side_size.x) / 2.0
            } else {
                center.x + (size.x - side_size.x) / 2.0
            };
            let side_z = center.y - main_size.y / 2.0;
            let side_pos = Vec3::new(side_x, terrain_height + height / 2.0, side_z);
            let side_scale = Vec3::new(side_size.x, height, side_size.y);

            let side_instance = BuildingInstanceData::new(
                side_pos,
                side_scale,
                Quat::IDENTITY,
                color,
                facade,
                archetype,
                shape,
                lot_index,
            );
            instance_buffer.add(side_instance);

            // Spawn L-shape using shared materials (two boxes)
            spawn_l_building_with_material(
                commands,
                mesh_pool.box_mesh.clone(),
                material.clone(),
                center,
                size,
                height,
                terrain_height,
                facade,
                lot_index,
                archetype,
                wing_ratio,
                rng.gen::<bool>(),
            );

            main_idx
        }
        BuildingShape::TowerOnBase => {
            // Tower on base - two instances (podium + tower)
            let base_height = 4.0 + rng.gen::<f32>() * 4.0;

            // Base podium
            let base_pos = Vec3::new(center.x, terrain_height + base_height / 2.0, center.y);
            let base_scale = Vec3::new(size.x, base_height, size.y);

            let base_instance = BuildingInstanceData::new(
                base_pos,
                base_scale,
                Quat::IDENTITY,
                color,
                facade,
                archetype,
                shape,
                lot_index,
            );
            let base_idx = instance_buffer.add(base_instance);

            // Tower on top
            let tower_ratio = 0.5 + rng.gen::<f32>() * 0.2;
            let tower_size = size * tower_ratio;
            let tower_height = height - base_height;

            if tower_height > 2.0 {
                let tower_pos = Vec3::new(
                    center.x,
                    terrain_height + base_height + tower_height / 2.0,
                    center.y,
                );
                let tower_scale = Vec3::new(tower_size.x, tower_height, tower_size.y);

                // Slightly different color for tower
                let tower_color = Color::srgb(0.5, 0.55, 0.65);

                let tower_instance = BuildingInstanceData::new(
                    tower_pos,
                    tower_scale,
                    Quat::IDENTITY,
                    tower_color,
                    FacadeStyle::Glass,
                    archetype,
                    shape,
                    lot_index,
                );
                instance_buffer.add(tower_instance);
            }

            // Get glass material for tower from palette
            let tower_material = palette.get(FacadeStyle::Glass, rng.gen_range(0..3))
                .expect("Material palette not initialized");

            // Spawn with shared materials (enables GPU instancing)
            spawn_tower_building_with_material(
                commands,
                mesh_pool.box_mesh.clone(),
                material.clone(),
                tower_material,
                center,
                size,
                height,
                terrain_height,
                facade,
                lot_index,
                archetype,
                base_height,
                tower_ratio,
            );

            base_idx
        }
        BuildingShape::Stepped => {
            // Stepped building - multiple instances
            let num_steps = 2 + rng.gen_range(0..2);
            let step_height = height / num_steps as f32;
            let mut first_idx = 0;

            for i in 0..num_steps {
                let shrink = 1.0 - (i as f32 * 0.15);
                let step_size = size * shrink;
                let y = terrain_height + step_height * i as f32 + step_height / 2.0;

                let step_pos = Vec3::new(center.x, y, center.y);
                let step_scale = Vec3::new(step_size.x, step_height, step_size.y);

                let step_instance = BuildingInstanceData::new(
                    step_pos,
                    step_scale,
                    Quat::IDENTITY,
                    color,
                    facade,
                    archetype,
                    shape,
                    lot_index,
                );

                let idx = instance_buffer.add(step_instance);
                if i == 0 {
                    first_idx = idx;
                }
            }

            // Spawn with shared material (enables GPU instancing)
            spawn_stepped_building_with_material(
                commands,
                mesh_pool.box_mesh.clone(),
                material.clone(),
                center,
                size,
                height,
                terrain_height,
                facade,
                lot_index,
                archetype,
                num_steps,
            );

            first_idx
        }
    }
}

/// Calculate bounding sphere radius from building dimensions.
/// Uses the diagonal of the bounding box as the sphere diameter.
fn calculate_bounding_radius(width: f32, height: f32, depth: f32) -> f32 {
    (width * width + height * height + depth * depth).sqrt() / 2.0
}

/// Spawn a box building using a shared material (enables GPU instancing).
fn spawn_box_building_with_material(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    facade: FacadeStyle,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    let bounding_radius = calculate_bounding_radius(size.x, height, size.y);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(center.x, terrain_height + height / 2.0, center.y)
            .with_scale(Vec3::new(size.x, height, size.y)),
        Building {
            lot_index,
            building_type,
            facade_style: facade,
        },
        GpuCullable::new(bounding_radius),
    ));
}

/// Spawn a box building using standard materials (legacy - creates unique material).
#[allow(dead_code)]
fn spawn_box_building_standard(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    color: Color,
    facade: FacadeStyle,
    lot_index: usize,
    building_type: BuildingArchetype,
) {
    let (roughness, metallic) = get_facade_material_params(facade);
    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: roughness,
        metallic,
        ..default()
    });

    let bounding_radius = calculate_bounding_radius(size.x, height, size.y);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(center.x, terrain_height + height / 2.0, center.y)
            .with_scale(Vec3::new(size.x, height, size.y)),
        Building {
            lot_index,
            building_type,
            facade_style: facade,
        },
        GpuCullable::new(bounding_radius),
    ));
}

/// Spawn an L-shaped building using shared material (enables GPU instancing).
fn spawn_l_building_with_material(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    facade: FacadeStyle,
    lot_index: usize,
    building_type: BuildingArchetype,
    wing_ratio: f32,
    side_left: bool,
) {
    // Main wing
    let main_size = Vec2::new(size.x, size.y * wing_ratio);
    let main_offset = Vec2::new(0.0, (size.y - main_size.y) / 2.0);
    let main_radius = calculate_bounding_radius(main_size.x, height, main_size.y);

    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(
            center.x + main_offset.x,
            terrain_height + height / 2.0,
            center.y + main_offset.y,
        )
        .with_scale(Vec3::new(main_size.x, height, main_size.y)),
        Building {
            lot_index,
            building_type,
            facade_style: facade,
        },
        GpuCullable::new(main_radius),
    ));

    // Side wing
    let side_size = Vec2::new(size.x * wing_ratio, size.y - main_size.y + 1.0);
    let side_x = if side_left {
        center.x - (size.x - side_size.x) / 2.0
    } else {
        center.x + (size.x - side_size.x) / 2.0
    };
    let side_z = center.y - main_size.y / 2.0;
    let side_radius = calculate_bounding_radius(side_size.x, height, side_size.y);

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(side_x, terrain_height + height / 2.0, side_z)
            .with_scale(Vec3::new(side_size.x, height, side_size.y)),
        Building {
            lot_index,
            building_type,
            facade_style: facade,
        },
        GpuCullable::new(side_radius),
    ));
}

/// Spawn a tower building using shared materials (enables GPU instancing).
fn spawn_tower_building_with_material(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    base_material: Handle<StandardMaterial>,
    tower_material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    facade: FacadeStyle,
    lot_index: usize,
    building_type: BuildingArchetype,
    base_height: f32,
    tower_ratio: f32,
) {
    // Base podium
    let base_radius = calculate_bounding_radius(size.x, base_height, size.y);
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(base_material),
        Transform::from_xyz(center.x, terrain_height + base_height / 2.0, center.y)
            .with_scale(Vec3::new(size.x, base_height, size.y)),
        Building {
            lot_index,
            building_type,
            facade_style: facade,
        },
        GpuCullable::new(base_radius),
    ));

    // Tower
    let tower_size = size * tower_ratio;
    let tower_height = height - base_height;

    if tower_height > 2.0 {
        let tower_radius = calculate_bounding_radius(tower_size.x, tower_height, tower_size.y);

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(tower_material),
            Transform::from_xyz(
                center.x,
                terrain_height + base_height + tower_height / 2.0,
                center.y,
            )
            .with_scale(Vec3::new(tower_size.x, tower_height, tower_size.y)),
            Building {
                lot_index,
                building_type,
                facade_style: FacadeStyle::Glass, // Tower is always glass
            },
            GpuCullable::new(tower_radius),
        ));
    }
}

/// Spawn a stepped building using shared material (enables GPU instancing).
fn spawn_stepped_building_with_material(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    size: Vec2,
    height: f32,
    terrain_height: f32,
    facade: FacadeStyle,
    lot_index: usize,
    building_type: BuildingArchetype,
    num_steps: usize,
) {
    let step_height = height / num_steps as f32;

    for i in 0..num_steps {
        let shrink = 1.0 - (i as f32 * 0.15);
        let step_size = size * shrink;
        let y = terrain_height + step_height * i as f32 + step_height / 2.0;
        let step_radius = calculate_bounding_radius(step_size.x, step_height, step_size.y);

        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(center.x, y, center.y)
                .with_scale(Vec3::new(step_size.x, step_height, step_size.y)),
            Building {
                lot_index,
                building_type,
                facade_style: facade,
            },
            GpuCullable::new(step_radius),
        ));
    }
}

/// Get material parameters for a facade style.
fn get_facade_material_params(facade: FacadeStyle) -> (f32, f32) {
    match facade {
        FacadeStyle::Brick => (0.9, 0.0),
        FacadeStyle::Concrete => (0.85, 0.0),
        FacadeStyle::Glass => (0.2, 0.1),
        FacadeStyle::Metal => (0.35, 0.6),
        FacadeStyle::Painted => (0.75, 0.0),
    }
}

// Old spawn functions removed - now using shared meshes via _standard variants

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

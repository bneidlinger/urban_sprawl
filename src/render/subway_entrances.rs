//! Subway station entrances scattered around commercial areas.
//!
//! Spawns distinctive subway entrance structures near commercial buildings
//! with stairs descending into the ground and signage.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct SubwayEntrancesPlugin;

impl Plugin for SubwayEntrancesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubwayEntranceConfig>()
            .init_resource::<SubwayEntrancesSpawned>()
            .add_systems(Update, spawn_subway_entrances.run_if(should_spawn_entrances));
    }
}

#[derive(Resource, Default)]
pub struct SubwayEntrancesSpawned(pub bool);

fn should_spawn_entrances(
    buildings_spawned: Res<BuildingsSpawned>,
    entrances_spawned: Res<SubwayEntrancesSpawned>,
) -> bool {
    buildings_spawned.0 && !entrances_spawned.0
}

/// Subway entrance marker component.
#[derive(Component)]
pub struct SubwayEntrance {
    pub station_name: u32,
}

#[derive(Resource)]
pub struct SubwayEntranceConfig {
    pub seed: u64,
    pub max_entrances: usize,
    pub min_spacing: f32,
    pub entrance_width: f32,
    pub entrance_depth: f32,
    pub canopy_height: f32,
}

impl Default for SubwayEntranceConfig {
    fn default() -> Self {
        Self {
            seed: 55555,
            max_entrances: 8,
            min_spacing: 100.0,
            entrance_width: 4.0,
            entrance_depth: 3.0,
            canopy_height: 3.0,
        }
    }
}

fn spawn_subway_entrances(
    mut commands: Commands,
    config: Res<SubwayEntranceConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<SubwayEntrancesSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut entrance_count = 0;
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Collect commercial building positions
    let mut commercial_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| b.building_type == BuildingArchetype::Commercial)
        .map(|(_, t)| t.translation)
        .collect();

    // Shuffle for randomness
    for i in (1..commercial_positions.len()).rev() {
        let j = rng.gen_range(0..=i);
        commercial_positions.swap(i, j);
    }

    // Materials
    let frame_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.28),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.5, 0.6, 0.5),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        metallic: 0.3,
        ..default()
    });

    let sign_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.2),
        emissive: LinearRgba::new(0.5, 0.1, 0.1, 1.0),
        ..default()
    });

    let stair_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.52),
        perceptual_roughness: 0.8,
        ..default()
    });

    // Meshes
    let canopy_mesh = meshes.add(Cuboid::new(config.entrance_width, 0.15, config.entrance_depth));
    let post_mesh = meshes.add(Cylinder::new(0.08, config.canopy_height));
    let sign_mesh = meshes.add(Cuboid::new(1.5, 0.8, 0.1));
    let stair_mesh = meshes.add(Cuboid::new(config.entrance_width * 0.9, 0.2, config.entrance_depth * 0.8));
    let rail_mesh = meshes.add(Cylinder::new(0.03, config.entrance_depth));

    for pos in commercial_positions {
        if entrance_count >= config.max_entrances {
            break;
        }

        // Offset from building
        let offset = Vec3::new(
            rng.gen_range(-8.0..8.0),
            0.0,
            rng.gen_range(-8.0..8.0),
        );
        let entrance_pos = Vec3::new(pos.x + offset.x, 0.0, pos.z + offset.z);

        // Check spacing
        let too_close = placed_positions
            .iter()
            .any(|p| p.distance(entrance_pos) < config.min_spacing);
        if too_close {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);

        commands.spawn((
            Transform::from_translation(entrance_pos).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            SubwayEntrance {
                station_name: entrance_count as u32,
            },
        )).with_children(|parent| {
            // Canopy/roof
            parent.spawn((
                Mesh3d(canopy_mesh.clone()),
                MeshMaterial3d(frame_material.clone()),
                Transform::from_xyz(0.0, config.canopy_height, 0.0),
            ));

            // Support posts
            for (x, z) in [
                (-config.entrance_width / 2.0 + 0.2, -config.entrance_depth / 2.0 + 0.2),
                (config.entrance_width / 2.0 - 0.2, -config.entrance_depth / 2.0 + 0.2),
            ] {
                parent.spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(frame_material.clone()),
                    Transform::from_xyz(x, config.canopy_height / 2.0, z),
                ));
            }

            // Metro sign on top
            parent.spawn((
                Mesh3d(sign_mesh.clone()),
                MeshMaterial3d(sign_material.clone()),
                Transform::from_xyz(0.0, config.canopy_height + 0.5, -config.entrance_depth / 2.0),
            ));

            // Stairs going down (represented as descending platforms)
            for i in 0..4 {
                let stair_y = -0.3 * (i as f32 + 1.0);
                let stair_z = config.entrance_depth * 0.2 * (i as f32);
                parent.spawn((
                    Mesh3d(stair_mesh.clone()),
                    MeshMaterial3d(stair_material.clone()),
                    Transform::from_xyz(0.0, stair_y, stair_z),
                ));
            }

            // Handrails
            for x in [-config.entrance_width / 2.0 + 0.3, config.entrance_width / 2.0 - 0.3] {
                parent.spawn((
                    Mesh3d(rail_mesh.clone()),
                    MeshMaterial3d(frame_material.clone()),
                    Transform::from_xyz(x, 0.9, config.entrance_depth / 4.0)
                        .with_rotation(Quat::from_rotation_x(-0.3)),
                ));
            }

            // Glass side panels
            let glass_panel = meshes.add(Cuboid::new(0.05, config.canopy_height * 0.7, config.entrance_depth * 0.8));
            for x in [-config.entrance_width / 2.0, config.entrance_width / 2.0] {
                parent.spawn((
                    Mesh3d(glass_panel.clone()),
                    MeshMaterial3d(glass_material.clone()),
                    Transform::from_xyz(x, config.canopy_height * 0.4, 0.0),
                ));
            }
        });

        placed_positions.push(entrance_pos);
        entrance_count += 1;
    }

    info!("Spawned {} subway entrances", entrance_count);
}

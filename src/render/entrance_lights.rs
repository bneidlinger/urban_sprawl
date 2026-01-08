//! Building entrance lights.
//!
//! Spawns small point lights at building entrances/doorways that illuminate at night.
//! Commercial buildings get brighter, cooler lights while residential get warmer, dimmer ones.

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};
use crate::render::clustered_shading::{ClusterConfig, DynamicCityLight};

pub struct EntranceLightsPlugin;

impl Plugin for EntranceLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntranceLightConfig>()
            .init_resource::<EntranceLightsSpawned>()
            .add_systems(Update, spawn_entrance_lights.run_if(should_spawn_lights));
    }
}

/// Marker resource to prevent entrance lights from spawning multiple times.
#[derive(Resource, Default)]
pub struct EntranceLightsSpawned(pub bool);

fn should_spawn_lights(
    spawned: Res<BuildingsSpawned>,
    entrance_spawned: Res<EntranceLightsSpawned>,
) -> bool {
    spawned.0 && !entrance_spawned.0
}

/// Component marking an entrance light entity.
#[derive(Component)]
pub struct EntranceLight {
    pub building_type: BuildingArchetype,
}

/// Configuration for entrance light spawning.
#[derive(Resource)]
pub struct EntranceLightConfig {
    pub seed: u64,
    /// Height above ground for entrance lights
    pub light_height: f32,
    /// Probability of entrance light per building face
    pub spawn_probability: f32,
    /// Maximum entrances per building
    pub max_entrances_per_building: usize,
    /// Commercial building light intensity
    pub commercial_intensity: f32,
    /// Residential building light intensity
    pub residential_intensity: f32,
    /// Industrial building light intensity
    pub industrial_intensity: f32,
    /// Light radius
    pub light_radius: f32,
}

impl Default for EntranceLightConfig {
    fn default() -> Self {
        Self {
            seed: 55555,
            light_height: 3.0,           // 3m above ground (above door frame)
            spawn_probability: 0.8,      // 80% of buildings get entrance lights
            max_entrances_per_building: 2,
            commercial_intensity: 8000.0, // Bright storefront lighting
            residential_intensity: 3000.0, // Subtle porch light
            industrial_intensity: 12000.0, // Bright security lighting
            light_radius: 12.0,          // 12m radius
        }
    }
}

/// Entrance light color based on building type.
fn get_entrance_light_color(building_type: BuildingArchetype) -> Color {
    match building_type {
        // Warm welcoming light for homes
        BuildingArchetype::Residential => Color::srgb(1.0, 0.9, 0.75),
        // Bright cool white for storefronts
        BuildingArchetype::Commercial => Color::srgb(1.0, 0.98, 0.95),
        // Harsh white/blue security lighting
        BuildingArchetype::Industrial => Color::srgb(0.95, 0.98, 1.0),
    }
}

fn spawn_entrance_lights(
    mut commands: Commands,
    config: Res<EntranceLightConfig>,
    cluster_config: Res<ClusterConfig>,
    building_query: Query<(&Building, &Transform, &Mesh3d), With<Building>>,
    meshes: Res<Assets<Mesh>>,
    mut spawned: ResMut<EntranceLightsSpawned>,
) {
    spawned.0 = true;

    info!("Spawning building entrance lights...");

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut light_count = 0;
    let mut residential_count = 0;
    let mut commercial_count = 0;
    let mut industrial_count = 0;

    for (building, transform, mesh_handle) in building_query.iter() {
        // Random chance for this building to have entrance lights
        if rng.gen::<f32>() > config.spawn_probability {
            continue;
        }

        // Get building dimensions from mesh AABB
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };

        // Apply transform scale to get world-space dimensions
        let scale = transform.scale;
        let building_width = aabb.half_extents.x * 2.0 * scale.x;
        let building_height = aabb.half_extents.y * 2.0 * scale.y;
        let building_depth = aabb.half_extents.z * 2.0 * scale.z;

        // Skip very small buildings
        if building_width < 3.0 || building_height < 4.0 {
            continue;
        }

        let pos = transform.translation;
        let building_base = pos.y - building_height / 2.0;
        let light_y = building_base + config.light_height;

        // Get light properties based on building type
        let light_color = get_entrance_light_color(building.building_type);
        let intensity = match building.building_type {
            BuildingArchetype::Residential => config.residential_intensity,
            BuildingArchetype::Commercial => config.commercial_intensity,
            BuildingArchetype::Industrial => config.industrial_intensity,
        };

        // Determine number of entrances (1-2 based on building size)
        let num_entrances = if building_width > 15.0 || building_depth > 15.0 {
            rng.gen_range(1..=config.max_entrances_per_building)
        } else {
            1
        };

        // Track which faces we've used
        let mut used_faces: Vec<usize> = Vec::new();

        for _ in 0..num_entrances {
            // Pick a face (0=front +Z, 1=back -Z, 2=left -X, 3=right +X)
            // Prefer front and side faces for entrances
            let face_weights = [0.4, 0.1, 0.25, 0.25]; // front, back, left, right
            let available_faces: Vec<(usize, f32)> = (0..4)
                .filter(|f| !used_faces.contains(f))
                .map(|f| (f, face_weights[f]))
                .collect();

            if available_faces.is_empty() {
                break;
            }

            // Weighted random selection
            let total_weight: f32 = available_faces.iter().map(|(_, w)| w).sum();
            let mut choice = rng.gen::<f32>() * total_weight;
            let mut face_idx = available_faces[0].0;
            for (idx, weight) in &available_faces {
                choice -= weight;
                if choice <= 0.0 {
                    face_idx = *idx;
                    break;
                }
            }
            used_faces.push(face_idx);

            // Calculate entrance position on the face
            let (face_width, entrance_pos) = match face_idx {
                0 => {
                    // Front face (+Z)
                    let offset_range = (building_width / 2.0 - 1.5).max(0.0);
                    let offset = if offset_range > 0.1 {
                        rng.gen_range(-offset_range..offset_range)
                    } else {
                        0.0
                    };
                    (
                        building_width,
                        Vec3::new(pos.x + offset, light_y, pos.z + building_depth / 2.0 + 0.3),
                    )
                }
                1 => {
                    // Back face (-Z)
                    let offset_range = (building_width / 2.0 - 1.5).max(0.0);
                    let offset = if offset_range > 0.1 {
                        rng.gen_range(-offset_range..offset_range)
                    } else {
                        0.0
                    };
                    (
                        building_width,
                        Vec3::new(pos.x + offset, light_y, pos.z - building_depth / 2.0 - 0.3),
                    )
                }
                2 => {
                    // Left face (-X)
                    let offset_range = (building_depth / 2.0 - 1.5).max(0.0);
                    let offset = if offset_range > 0.1 {
                        rng.gen_range(-offset_range..offset_range)
                    } else {
                        0.0
                    };
                    (
                        building_depth,
                        Vec3::new(pos.x - building_width / 2.0 - 0.3, light_y, pos.z + offset),
                    )
                }
                _ => {
                    // Right face (+X)
                    let offset_range = (building_depth / 2.0 - 1.5).max(0.0);
                    let offset = if offset_range > 0.1 {
                        rng.gen_range(-offset_range..offset_range)
                    } else {
                        0.0
                    };
                    (
                        building_depth,
                        Vec3::new(pos.x + building_width / 2.0 + 0.3, light_y, pos.z + offset),
                    )
                }
            };

            // Skip if face is too narrow for an entrance
            if face_width < 3.0 {
                continue;
            }

            // Spawn the entrance light as a real PointLight
            commands.spawn((
                PointLight {
                    color: light_color,
                    intensity: 0.0, // Managed by DynamicCityLight
                    range: config.light_radius,
                    radius: 0.3,
                    shadows_enabled: cluster_config.point_light_shadows,
                    ..default()
                },
                Transform::from_translation(entrance_pos),
                DynamicCityLight::entrance_light(intensity),
                EntranceLight {
                    building_type: building.building_type,
                },
            ));

            light_count += 1;
            match building.building_type {
                BuildingArchetype::Residential => residential_count += 1,
                BuildingArchetype::Commercial => commercial_count += 1,
                BuildingArchetype::Industrial => industrial_count += 1,
            }
        }
    }

    info!(
        "Spawned {} entrance lights ({} residential, {} commercial, {} industrial)",
        light_count, residential_count, commercial_count, industrial_count
    );
}

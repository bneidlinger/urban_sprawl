//! Street lamp generation along roads.

use bevy::prelude::*;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;
use crate::render::day_night::TimeOfDay;

pub struct StreetLampsPlugin;

impl Plugin for StreetLampsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LampConfig>()
            .add_systems(Update, spawn_street_lamps.run_if(should_spawn_lamps))
            .add_systems(Update, update_lamp_brightness);
    }
}

fn should_spawn_lamps(
    road_mesh_query: Query<&RoadMeshGenerated>,
    lamp_query: Query<&StreetLamp>,
) -> bool {
    !road_mesh_query.is_empty() && lamp_query.is_empty()
}

#[derive(Component)]
pub struct StreetLamp;

#[derive(Component)]
pub struct LampFixture;

#[derive(Resource)]
pub struct LampConfig {
    pub spacing: f32,
    pub pole_height: f32,
    pub pole_radius: f32,
    pub light_radius: f32,
    pub offset_from_road: f32,
}

impl Default for LampConfig {
    fn default() -> Self {
        Self {
            spacing: 30.0,        // ~30m between lamps (realistic)
            pole_height: 8.0,     // 8m tall (realistic street lamp)
            pole_radius: 0.15,
            light_radius: 0.5,
            offset_from_road: 5.0,
        }
    }
}

fn spawn_street_lamps(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<LampConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning street lamps...");

    // Pole material (dark metal)
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.6,
        metallic: 0.4,
        ..default()
    });

    // Light fixture material (glowing warm white)
    let light_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.8),
        emissive: LinearRgba::new(1.0, 0.9, 0.7, 1.0),
        ..default()
    });

    // Meshes
    let pole_mesh = meshes.add(Cylinder::new(config.pole_radius, config.pole_height));
    let light_mesh = meshes.add(Sphere::new(config.light_radius));

    let mut lamp_count = 0;

    for edge in road_graph.edges() {
        // Only add lamps to major roads
        if edge.road_type != RoadType::Major {
            continue;
        }

        if edge.points.len() < 2 {
            continue;
        }

        // Calculate road width for offset
        let road_width = match edge.road_type {
            RoadType::Highway => 12.0,
            RoadType::Major => 8.0,
            RoadType::Minor => 5.0,
            RoadType::Alley => 3.0,
        };

        let lamp_offset = road_width / 2.0 + config.offset_from_road;

        // Walk along the road and place lamps at intervals
        let mut accumulated_dist = config.spacing / 2.0; // Start offset
        let mut segment_start_dist = 0.0;

        for window in edge.points.windows(2) {
            let start = window[0];
            let end = window[1];
            let segment_length = start.distance(end);
            let segment_end_dist = segment_start_dist + segment_length;

            let dir = (end - start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);

            // Place lamps within this segment
            while accumulated_dist < segment_end_dist {
                let t = (accumulated_dist - segment_start_dist) / segment_length;
                let pos = start.lerp(end, t);

                // Alternate sides
                let side = if (lamp_count % 2) == 0 { 1.0 } else { -1.0 };
                let lamp_pos = pos + perp * lamp_offset * side;

                // Spawn pole
                commands.spawn((
                    Mesh3d(pole_mesh.clone()),
                    MeshMaterial3d(pole_material.clone()),
                    Transform::from_xyz(lamp_pos.x, config.pole_height / 2.0, lamp_pos.y),
                    StreetLamp,
                ));

                // Spawn light fixture
                commands.spawn((
                    Mesh3d(light_mesh.clone()),
                    MeshMaterial3d(light_material.clone()),
                    Transform::from_xyz(lamp_pos.x, config.pole_height + config.light_radius * 0.5, lamp_pos.y),
                    StreetLamp,
                    LampFixture,
                ));

                lamp_count += 1;
                accumulated_dist += config.spacing;
            }

            segment_start_dist = segment_end_dist;
        }
    }

    info!("Spawned {} street lamps", lamp_count);
}

fn update_lamp_brightness(
    tod: Res<TimeOfDay>,
    lamp_query: Query<&MeshMaterial3d<StandardMaterial>, With<LampFixture>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Calculate night factor (0 during day, 1 at night)
    let hour = tod.hour();
    let night_factor = if hour >= 6.0 && hour <= 7.0 {
        // Dawn - lamps turning off
        1.0 - (hour - 6.0)
    } else if hour >= 18.0 && hour <= 19.0 {
        // Dusk - lamps turning on
        hour - 18.0
    } else if hour > 7.0 && hour < 18.0 {
        // Day - lamps off (but not completely)
        0.1
    } else {
        // Night - lamps fully on
        1.0
    };

    for material_handle in lamp_query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Warm orange glow
            material.emissive = LinearRgba::new(
                1.0 * night_factor * 5.0,
                0.85 * night_factor * 5.0,
                0.5 * night_factor * 5.0,
                1.0,
            );
        }
    }
}

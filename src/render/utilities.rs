//! Urban utilities: power lines, manholes, storm drains, utility boxes.
//!
//! Spawns utility infrastructure throughout the city.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::road_mesh::RoadMeshGenerated;

pub struct UtilitiesPlugin;

impl Plugin for UtilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UtilitiesConfig>()
            .init_resource::<UtilitiesSpawned>()
            .add_systems(Update, spawn_utilities.run_if(should_spawn_utilities));
    }
}

#[derive(Resource, Default)]
pub struct UtilitiesSpawned(pub bool);

fn should_spawn_utilities(
    road_mesh: Query<&RoadMeshGenerated>,
    spawned: Res<UtilitiesSpawned>,
) -> bool {
    !road_mesh.is_empty() && !spawned.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UtilityType {
    PowerPole,
    Manhole,
    StormDrain,
    UtilityBox,
}

/// Marker component for utilities.
#[derive(Component)]
pub struct Utility {
    pub utility_type: UtilityType,
}

#[derive(Resource)]
pub struct UtilitiesConfig {
    pub seed: u64,
    pub power_pole_spacing: f32,
    pub manhole_spacing: f32,
    pub drain_spacing: f32,
    pub utility_box_count: usize,
}

impl Default for UtilitiesConfig {
    fn default() -> Self {
        Self {
            seed: 44444,
            power_pole_spacing: 40.0,
            manhole_spacing: 50.0,
            drain_spacing: 30.0,
            utility_box_count: 25,
        }
    }
}

fn spawn_utilities(
    mut commands: Commands,
    config: Res<UtilitiesConfig>,
    road_graph: Res<RoadGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<UtilitiesSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);

    // Materials
    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });

    let metal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });

    let concrete_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.48),
        perceptual_roughness: 0.9,
        ..default()
    });

    let rust_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.25),
        metallic: 0.4,
        perceptual_roughness: 0.8,
        ..default()
    });

    let green_box_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.25),
        perceptual_roughness: 0.6,
        ..default()
    });

    // Meshes
    let pole_mesh = meshes.add(Cylinder::new(0.12, 8.0));
    let crossarm_mesh = meshes.add(Cuboid::new(2.5, 0.1, 0.15));
    let insulator_mesh = meshes.add(Cylinder::new(0.04, 0.15));
    let wire_mesh = meshes.add(Cylinder::new(0.01, 1.0));

    let manhole_cover_mesh = meshes.add(Cylinder::new(0.4, 0.05));
    let manhole_rim_mesh = meshes.add(Torus::new(0.05, 0.45));

    let drain_grate_mesh = meshes.add(Cuboid::new(0.5, 0.03, 0.3));
    let drain_frame_mesh = meshes.add(Cuboid::new(0.55, 0.05, 0.35));

    let utility_box_mesh = meshes.add(Cuboid::new(0.6, 1.2, 0.4));

    let mut power_pole_count = 0;
    let mut manhole_count = 0;
    let mut drain_count = 0;
    let mut utility_box_count = 0;

    // Track placed positions to avoid clustering
    let mut pole_positions: Vec<Vec2> = Vec::new();
    let mut manhole_positions: Vec<Vec2> = Vec::new();
    let mut box_positions: Vec<Vec2> = Vec::new();

    // Iterate over road edges
    for edge in road_graph.edges() {
        let is_minor = matches!(edge.road_type, RoadType::Minor | RoadType::Alley);
        let is_major = matches!(edge.road_type, RoadType::Major);

        // Power poles on minor roads (suburban feel)
        if is_minor && edge.length >= config.power_pole_spacing {
            let num_poles = (edge.length / config.power_pole_spacing).floor() as i32;
            for i in 1..num_poles {
                let t = i as f32 / num_poles as f32;
                let (pos, dir) = interpolate_edge(&edge.points, t);
                let perp = Vec2::new(-dir.y, dir.x);
                let pole_pos = pos + perp * 6.0;

                // Check spacing
                if pole_positions.iter().any(|p| p.distance(pole_pos) < config.power_pole_spacing * 0.8) {
                    continue;
                }

                let angle = dir.y.atan2(dir.x);
                spawn_power_pole(
                    &mut commands,
                    &pole_mesh,
                    &crossarm_mesh,
                    &insulator_mesh,
                    &wood_material,
                    &metal_material,
                    pole_pos,
                    angle,
                );

                pole_positions.push(pole_pos);
                power_pole_count += 1;
            }
        }

        // Manholes on major roads
        if is_major && edge.length >= config.manhole_spacing {
            let num_manholes = (edge.length / config.manhole_spacing).floor() as i32;
            for i in 1..num_manholes {
                let t = i as f32 / num_manholes as f32;
                let (pos, _) = interpolate_edge(&edge.points, t);

                if manhole_positions.iter().any(|p| p.distance(pos) < config.manhole_spacing * 0.6) {
                    continue;
                }

                commands
                    .spawn((
                        Transform::from_xyz(pos.x, 0.01, pos.y),
                        GlobalTransform::default(),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                        Utility { utility_type: UtilityType::Manhole },
                    ))
                    .with_children(|parent| {
                        // Cover
                        parent.spawn((
                            Mesh3d(manhole_cover_mesh.clone()),
                            MeshMaterial3d(rust_material.clone()),
                            Transform::from_xyz(0.0, 0.025, 0.0),
                        ));

                        // Rim
                        parent.spawn((
                            Mesh3d(manhole_rim_mesh.clone()),
                            MeshMaterial3d(concrete_material.clone()),
                            Transform::from_xyz(0.0, 0.0, 0.0)
                                .with_rotation(Quat::from_rotation_x(PI / 2.0)),
                        ));
                    });

                manhole_positions.push(pos);
                manhole_count += 1;
            }
        }

        // Storm drains along curbs
        if edge.length >= config.drain_spacing {
            let num_drains = (edge.length / config.drain_spacing).floor() as i32;
            for i in 1..num_drains {
                if rng.gen::<f32>() > 0.6 {
                    continue;
                }

                let t = i as f32 / num_drains as f32;
                let (pos, dir) = interpolate_edge(&edge.points, t);
                let perp = Vec2::new(-dir.y, dir.x);

                // Place at curb (sidewalk edge)
                let side = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                let drain_pos = pos + perp * 4.5 * side;
                let angle = dir.y.atan2(dir.x);

                commands
                    .spawn((
                        Transform::from_xyz(drain_pos.x, 0.01, drain_pos.y)
                            .with_rotation(Quat::from_rotation_y(-angle)),
                        GlobalTransform::default(),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                        Utility { utility_type: UtilityType::StormDrain },
                    ))
                    .with_children(|parent| {
                        // Frame
                        parent.spawn((
                            Mesh3d(drain_frame_mesh.clone()),
                            MeshMaterial3d(concrete_material.clone()),
                            Transform::from_xyz(0.0, 0.0, 0.0),
                        ));

                        // Grate
                        parent.spawn((
                            Mesh3d(drain_grate_mesh.clone()),
                            MeshMaterial3d(rust_material.clone()),
                            Transform::from_xyz(0.0, 0.03, 0.0),
                        ));
                    });

                drain_count += 1;
            }
        }
    }

    // Utility boxes near intersections
    for (node_idx, node) in road_graph.nodes() {
        if utility_box_count >= config.utility_box_count {
            break;
        }

        let neighbor_count = road_graph.neighbors(node_idx).count();
        if neighbor_count < 2 || rng.gen::<f32>() > 0.15 {
            continue;
        }

        let offset = Vec2::new(rng.gen_range(-8.0..8.0), rng.gen_range(-8.0..8.0));
        let box_pos = node.position + offset;

        if box_positions.iter().any(|p| p.distance(box_pos) < 20.0) {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);

        commands
            .spawn((
                Transform::from_xyz(box_pos.x, 0.0, box_pos.y).with_rotation(rotation),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Utility { utility_type: UtilityType::UtilityBox },
            ))
            .with_children(|parent| {
                // Main box
                parent.spawn((
                    Mesh3d(utility_box_mesh.clone()),
                    MeshMaterial3d(green_box_material.clone()),
                    Transform::from_xyz(0.0, 0.6, 0.0),
                ));

                // Ventilation slits (decorative)
                let slit_mesh = meshes.add(Cuboid::new(0.3, 0.02, 0.05));
                for y in [0.4, 0.5, 0.6] {
                    parent.spawn((
                        Mesh3d(slit_mesh.clone()),
                        MeshMaterial3d(metal_material.clone()),
                        Transform::from_xyz(0.0, y, 0.22),
                    ));
                }
            });

        box_positions.push(box_pos);
        utility_box_count += 1;
    }

    info!(
        "Spawned {} power poles, {} manholes, {} storm drains, {} utility boxes",
        power_pole_count, manhole_count, drain_count, utility_box_count
    );
}

fn spawn_power_pole(
    commands: &mut Commands,
    pole_mesh: &Handle<Mesh>,
    crossarm_mesh: &Handle<Mesh>,
    insulator_mesh: &Handle<Mesh>,
    wood_material: &Handle<StandardMaterial>,
    metal_material: &Handle<StandardMaterial>,
    position: Vec2,
    angle: f32,
) {
    commands
        .spawn((
            Transform::from_xyz(position.x, 0.0, position.y)
                .with_rotation(Quat::from_rotation_y(-angle)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Utility { utility_type: UtilityType::PowerPole },
        ))
        .with_children(|parent| {
            // Main pole
            parent.spawn((
                Mesh3d(pole_mesh.clone()),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(0.0, 4.0, 0.0),
            ));

            // Crossarm
            parent.spawn((
                Mesh3d(crossarm_mesh.clone()),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(0.0, 7.5, 0.0),
            ));

            // Insulators
            for x in [-1.0, 0.0, 1.0] {
                parent.spawn((
                    Mesh3d(insulator_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_xyz(x, 7.6, 0.0),
                ));
            }

            // Secondary crossarm (lower)
            parent.spawn((
                Mesh3d(crossarm_mesh.clone()),
                MeshMaterial3d(wood_material.clone()),
                Transform::from_xyz(0.0, 6.5, 0.0)
                    .with_scale(Vec3::new(0.7, 1.0, 1.0)),
            ));

            // More insulators
            for x in [-0.6, 0.6] {
                parent.spawn((
                    Mesh3d(insulator_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_xyz(x, 6.6, 0.0),
                ));
            }
        });
}

fn interpolate_edge(points: &[Vec2], t: f32) -> (Vec2, Vec2) {
    if points.is_empty() {
        return (Vec2::ZERO, Vec2::X);
    }
    if points.len() == 1 {
        return (points[0], Vec2::X);
    }

    let total_len: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    if total_len <= 0.0 {
        return (points[0], Vec2::X);
    }

    let target = t.clamp(0.0, 1.0) * total_len;
    let mut acc = 0.0;

    for w in points.windows(2) {
        let seg_len = w[0].distance(w[1]);
        if acc + seg_len >= target {
            let local_t = if seg_len > 0.0 { (target - acc) / seg_len } else { 0.0 };
            let pos = w[0].lerp(w[1], local_t);
            let dir = (w[1] - w[0]).normalize_or_zero();
            return (pos, dir);
        }
        acc += seg_len;
    }

    let last = *points.last().unwrap();
    let dir = if points.len() >= 2 {
        (points[points.len() - 1] - points[points.len() - 2]).normalize_or_zero()
    } else {
        Vec2::X
    };
    (last, dir)
}

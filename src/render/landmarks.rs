//! Landmark buildings: clock towers, churches, and other distinctive structures.
//!
//! Spawns unique landmark buildings at strategic locations to provide
//! visual focal points throughout the city.

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::{BuildingArchetype, BuildingBlueprints, BuildingPlan, PlannedStructure};
use crate::render::building_spawner::BuildingsSpawned;

pub struct LandmarksPlugin;

impl Plugin for LandmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LandmarkConfig>()
            .init_resource::<LandmarksSpawned>()
            .add_systems(Update, spawn_landmarks.run_if(should_spawn_landmarks));
    }
}

/// Marker resource to prevent landmark system from running multiple times.
#[derive(Resource, Default)]
pub struct LandmarksSpawned(pub bool);

fn should_spawn_landmarks(
    buildings_spawned: Res<BuildingsSpawned>,
    landmarks_spawned: Res<LandmarksSpawned>,
) -> bool {
    buildings_spawned.0 && !landmarks_spawned.0
}

/// Generic landmark marker.
#[derive(Component)]
pub struct Landmark;

/// Clock tower marker.
#[derive(Component)]
pub struct ClockTower;

/// Church/chapel marker.
#[derive(Component)]
pub struct Church;

/// Configuration for landmark spawning.
#[derive(Resource)]
pub struct LandmarkConfig {
    pub seed: u64,
    /// Number of clock towers to spawn (1-2).
    pub clock_tower_count: u32,
    /// Number of churches to spawn (2-4).
    pub church_count: u32,
    /// Minimum distance between churches.
    pub church_min_spacing: f32,
}

impl Default for LandmarkConfig {
    fn default() -> Self {
        Self {
            seed: 77777,
            clock_tower_count: 2,
            church_count: 3,
            church_min_spacing: 80.0,
        }
    }
}

fn spawn_landmarks(
    mut commands: Commands,
    config: Res<LandmarkConfig>,
    blueprints: Res<BuildingBlueprints>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut landmarks_spawned: ResMut<LandmarksSpawned>,
) {
    landmarks_spawned.0 = true;

    if !blueprints.generated {
        info!("Landmarks: No blueprints generated, skipping");
        return;
    }

    info!("Spawning landmark buildings...");
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Create materials for landmarks
    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.78, 0.68), // Warm tan stone
        perceptual_roughness: 0.75,
        ..default()
    });

    let dark_stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.48, 0.45), // Darker stone for details
        perceptual_roughness: 0.7,
        ..default()
    });

    let copper_spire_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.55, 0.45), // Copper patina green
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let clock_face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.93, 0.88), // Off-white clock face
        perceptual_roughness: 0.3,
        ..default()
    });

    let slate_roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.4), // Dark slate
        perceptual_roughness: 0.8,
        ..default()
    });

    let white_trim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.92), // White trim
        perceptual_roughness: 0.5,
        ..default()
    });

    let church_stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.78, 0.75), // Light gray stone
        perceptual_roughness: 0.75,
        ..default()
    });

    // Find all building plans for church placement - prefer residential but allow any
    let mut all_plans: Vec<&BuildingPlan> = blueprints
        .plans
        .iter()
        .filter_map(|p| match p {
            PlannedStructure::Building(plan) => Some(plan),
            _ => None,
        })
        .collect();

    // Sort by footprint size (larger lots preferred for churches)
    all_plans.sort_by(|a, b| {
        let area_a = a.footprint.x * a.footprint.y;
        let area_b = b.footprint.x * b.footprint.y;
        area_b.partial_cmp(&area_a).unwrap()
    });

    info!(
        "Found {} building plans, largest footprint: {:?}",
        all_plans.len(),
        all_plans.first().map(|p| p.footprint)
    );

    // Take top 30% largest lots as candidates
    let top_count = (all_plans.len() / 3).max(10);
    let residential_plans: Vec<&BuildingPlan> = all_plans.into_iter().take(top_count).collect();

    info!("Using top {} largest lots as church candidates", residential_plans.len());

    // Spawn clock towers near city center
    let mut clock_towers_spawned = 0;

    // Find positions near city center that have some clearance
    // We'll place them at strategic positions near (0,0)
    let clock_tower_positions = find_clock_tower_positions(
        &blueprints,
        config.clock_tower_count as usize,
        &mut rng,
    );

    for pos in clock_tower_positions {
        spawn_clock_tower(
            &mut commands,
            &mut meshes,
            pos,
            &stone_material,
            &dark_stone_material,
            &copper_spire_material,
            &clock_face_material,
        );
        clock_towers_spawned += 1;
    }

    // Spawn churches in residential areas
    let mut churches_spawned = 0;
    let mut church_positions: Vec<Vec2> = Vec::new();

    // Use all candidates - no distance filtering needed since we already selected large lots
    let mut candidate_plans: Vec<&BuildingPlan> = residential_plans;

    // Shuffle for variety
    for i in (1..candidate_plans.len()).rev() {
        let j = rng.gen_range(0..=i);
        candidate_plans.swap(i, j);
    }

    for plan in candidate_plans.iter() {
        if churches_spawned >= config.church_count {
            break;
        }

        // Check spacing from other churches
        let too_close = church_positions.iter().any(|&pos| {
            (pos - plan.center).length() < config.church_min_spacing
        });

        if too_close {
            continue;
        }

        spawn_church(
            &mut commands,
            &mut meshes,
            plan.center,
            &church_stone_material,
            &slate_roof_material,
            &white_trim_material,
            &copper_spire_material,
            &mut rng,
        );

        church_positions.push(plan.center);
        churches_spawned += 1;
    }

    info!(
        "Spawned {} clock towers and {} churches",
        clock_towers_spawned, churches_spawned
    );
}

/// Find suitable positions for clock towers near city center.
fn find_clock_tower_positions(
    blueprints: &BuildingBlueprints,
    count: usize,
    rng: &mut StdRng,
) -> Vec<Vec2> {
    let mut positions = Vec::new();

    // Get all building centers to find gaps
    let building_centers: Vec<Vec2> = blueprints
        .plans
        .iter()
        .filter_map(|p| match p {
            PlannedStructure::Building(plan) => Some(plan.center),
            PlannedStructure::Park(park) => Some(park.center),
        })
        .collect();

    // Find the closest buildings to center and place clock towers nearby
    let mut near_center: Vec<Vec2> = building_centers
        .iter()
        .filter(|c| c.length() < 100.0)
        .copied()
        .collect();

    near_center.sort_by(|a, b| a.length().partial_cmp(&b.length()).unwrap());

    // Place clock towers offset from the nearest buildings
    for i in 0..count.min(near_center.len().max(1)) {
        let base_pos = if !near_center.is_empty() {
            near_center[i.min(near_center.len() - 1)]
        } else {
            Vec2::ZERO
        };

        // Offset position to find a clear spot
        let angle = rng.gen::<f32>() * 2.0 * PI;
        let offset_dist = 15.0 + rng.gen::<f32>() * 10.0;
        let offset = Vec2::new(angle.cos(), angle.sin()) * offset_dist;

        let final_pos = base_pos + offset;

        // Make sure we're not overlapping with existing buildings
        let is_clear = building_centers
            .iter()
            .all(|&c| (c - final_pos).length() > 8.0);

        if is_clear {
            positions.push(final_pos);
        } else {
            // Try the base position with smaller offset
            positions.push(base_pos + Vec2::new(5.0, 5.0));
        }
    }

    // If we couldn't find any, just place near origin
    if positions.is_empty() {
        positions.push(Vec2::new(10.0, 10.0));
    }

    positions
}

/// Spawn a clock tower at the given position.
fn spawn_clock_tower(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Vec2,
    stone_material: &Handle<StandardMaterial>,
    dark_stone_material: &Handle<StandardMaterial>,
    copper_material: &Handle<StandardMaterial>,
    clock_face_material: &Handle<StandardMaterial>,
) {
    let base_y = 0.0; // TODO: Sample terrain height

    // Tower dimensions
    let tower_width = 5.0;
    let tower_height = 28.0;
    let deck_height = 2.5;
    let deck_width = 6.5;
    let spire_height = 10.0;
    let clock_size = 2.8;

    // Main tower body
    let tower_mesh = meshes.add(Cuboid::new(tower_width, tower_height, tower_width));
    commands.spawn((
        Mesh3d(tower_mesh),
        MeshMaterial3d(stone_material.clone()),
        Transform::from_xyz(position.x, base_y + tower_height / 2.0, position.y),
        Landmark,
        ClockTower,
    ));

    // Observation deck / cornice at top
    let deck_mesh = meshes.add(Cuboid::new(deck_width, deck_height, deck_width));
    commands.spawn((
        Mesh3d(deck_mesh),
        MeshMaterial3d(dark_stone_material.clone()),
        Transform::from_xyz(position.x, base_y + tower_height + deck_height / 2.0, position.y),
        Landmark,
        ClockTower,
    ));

    // Spire (pyramid/cone shape using scaled cube)
    // Use a tall thin box as approximation, or create a cone
    let spire_mesh = meshes.add(Cone {
        radius: deck_width / 2.0 * 0.7,
        height: spire_height,
    });
    commands.spawn((
        Mesh3d(spire_mesh),
        MeshMaterial3d(copper_material.clone()),
        Transform::from_xyz(
            position.x,
            base_y + tower_height + deck_height + spire_height / 2.0,
            position.y,
        ),
        Landmark,
        ClockTower,
    ));

    // Clock faces on all 4 sides
    let clock_mesh = meshes.add(Cuboid::new(clock_size, clock_size, 0.15));
    let clock_y = base_y + tower_height - 4.0; // Near top of tower
    let clock_offset = tower_width / 2.0 + 0.1;

    // North face
    commands.spawn((
        Mesh3d(clock_mesh.clone()),
        MeshMaterial3d(clock_face_material.clone()),
        Transform::from_xyz(position.x, clock_y, position.y + clock_offset),
        Landmark,
        ClockTower,
    ));

    // South face
    commands.spawn((
        Mesh3d(clock_mesh.clone()),
        MeshMaterial3d(clock_face_material.clone()),
        Transform::from_xyz(position.x, clock_y, position.y - clock_offset),
        Landmark,
        ClockTower,
    ));

    // East face
    commands.spawn((
        Mesh3d(clock_mesh.clone()),
        MeshMaterial3d(clock_face_material.clone()),
        Transform::from_xyz(position.x + clock_offset, clock_y, position.y)
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
        Landmark,
        ClockTower,
    ));

    // West face
    commands.spawn((
        Mesh3d(clock_mesh.clone()),
        MeshMaterial3d(clock_face_material.clone()),
        Transform::from_xyz(position.x - clock_offset, clock_y, position.y)
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
        Landmark,
        ClockTower,
    ));

    // Small windows on tower (decorative bands)
    let window_band_mesh = meshes.add(Cuboid::new(tower_width + 0.2, 0.8, tower_width + 0.2));
    for i in 0..3 {
        let band_y = base_y + 8.0 + (i as f32 * 8.0);
        commands.spawn((
            Mesh3d(window_band_mesh.clone()),
            MeshMaterial3d(dark_stone_material.clone()),
            Transform::from_xyz(position.x, band_y, position.y),
            Landmark,
            ClockTower,
        ));
    }
}

/// Spawn a church at the given position.
fn spawn_church(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Vec2,
    stone_material: &Handle<StandardMaterial>,
    roof_material: &Handle<StandardMaterial>,
    trim_material: &Handle<StandardMaterial>,
    copper_material: &Handle<StandardMaterial>,
    rng: &mut StdRng,
) {
    let base_y = 0.0; // TODO: Sample terrain height

    // Randomize orientation (facing one of 4 cardinal directions)
    let rotation_angle = (rng.gen_range(0..4) as f32) * PI / 2.0;
    let base_rotation = Quat::from_rotation_y(rotation_angle);

    // Nave dimensions
    let nave_length = 22.0;
    let nave_width = 12.0;
    let nave_height = 10.0;
    let roof_height = 5.0;

    // Bell tower dimensions
    let tower_size = 5.0;
    let tower_height = 22.0;
    let steeple_height = 10.0;

    // Nave (main building)
    let nave_mesh = meshes.add(Cuboid::new(nave_width, nave_height, nave_length));
    commands.spawn((
        Mesh3d(nave_mesh),
        MeshMaterial3d(stone_material.clone()),
        Transform::from_xyz(position.x, base_y + nave_height / 2.0, position.y)
            .with_rotation(base_rotation),
        Landmark,
        Church,
    ));

    // Pitched roof (using a triangular prism approximation - a rotated/scaled box for now)
    // Create a proper triangular prism mesh
    let roof_mesh = meshes.add(create_pitched_roof_mesh(nave_width, roof_height, nave_length));
    commands.spawn((
        Mesh3d(roof_mesh),
        MeshMaterial3d(roof_material.clone()),
        Transform::from_xyz(position.x, base_y + nave_height + roof_height / 2.0, position.y)
            .with_rotation(base_rotation),
        Landmark,
        Church,
    ));

    // Bell tower at front of church
    let tower_offset = Vec3::new(0.0, 0.0, -nave_length / 2.0 - tower_size / 2.0 + 2.0);
    let rotated_offset = base_rotation * tower_offset;
    let tower_pos = Vec3::new(position.x, base_y, position.y) + rotated_offset;

    let tower_mesh = meshes.add(Cuboid::new(tower_size, tower_height, tower_size));
    commands.spawn((
        Mesh3d(tower_mesh),
        MeshMaterial3d(stone_material.clone()),
        Transform::from_xyz(tower_pos.x, tower_pos.y + tower_height / 2.0, tower_pos.z),
        Landmark,
        Church,
    ));

    // Steeple on bell tower
    let steeple_mesh = meshes.add(Cone {
        radius: tower_size / 2.0 * 0.85,
        height: steeple_height,
    });
    commands.spawn((
        Mesh3d(steeple_mesh),
        MeshMaterial3d(copper_material.clone()),
        Transform::from_xyz(
            tower_pos.x,
            tower_pos.y + tower_height + steeple_height / 2.0,
            tower_pos.z,
        ),
        Landmark,
        Church,
    ));

    // Cross on top of steeple
    let cross_vertical = meshes.add(Cuboid::new(0.3, 2.5, 0.3));
    let cross_horizontal = meshes.add(Cuboid::new(1.5, 0.3, 0.3));
    let cross_y = tower_pos.y + tower_height + steeple_height + 1.0;

    commands.spawn((
        Mesh3d(cross_vertical),
        MeshMaterial3d(trim_material.clone()),
        Transform::from_xyz(tower_pos.x, cross_y, tower_pos.z),
        Landmark,
        Church,
    ));

    commands.spawn((
        Mesh3d(cross_horizontal),
        MeshMaterial3d(trim_material.clone()),
        Transform::from_xyz(tower_pos.x, cross_y + 0.5, tower_pos.z),
        Landmark,
        Church,
    ));

    // Rose window marker (circular detail on front facade)
    let rose_window_mesh = meshes.add(Cylinder::new(1.8, 0.2));
    let rose_offset = Vec3::new(0.0, nave_height - 2.0, -nave_length / 2.0 - 0.15);
    let rotated_rose = base_rotation * rose_offset;
    let rose_pos = Vec3::new(position.x, base_y, position.y) + rotated_rose;

    commands.spawn((
        Mesh3d(rose_window_mesh),
        MeshMaterial3d(trim_material.clone()),
        Transform::from_xyz(rose_pos.x, rose_pos.y, rose_pos.z)
            .with_rotation(base_rotation * Quat::from_rotation_x(PI / 2.0)),
        Landmark,
        Church,
    ));

    // Entrance (arched doorway represented by darker inset)
    let entrance_mesh = meshes.add(Cuboid::new(2.5, 4.0, 0.3));
    let entrance_offset = Vec3::new(0.0, 2.0, -nave_length / 2.0 - 0.2);
    let rotated_entrance = base_rotation * entrance_offset;
    let entrance_pos = Vec3::new(position.x, base_y, position.y) + rotated_entrance;

    commands.spawn((
        Mesh3d(entrance_mesh),
        MeshMaterial3d(roof_material.clone()), // Dark like the roof
        Transform::from_xyz(entrance_pos.x, entrance_pos.y, entrance_pos.z)
            .with_rotation(base_rotation),
        Landmark,
        Church,
    ));
}

/// Create a simple pitched roof mesh (triangular prism).
fn create_pitched_roof_mesh(width: f32, height: f32, length: f32) -> Mesh {
    let hw = width / 2.0;
    let hh = height / 2.0;
    let hl = length / 2.0;

    // Vertices for a triangular prism roof
    // The roof runs along the Z axis
    let positions = vec![
        // Front triangle
        [-hw, -hh, -hl],  // 0: bottom left
        [hw, -hh, -hl],   // 1: bottom right
        [0.0, hh, -hl],   // 2: top center
        // Back triangle
        [-hw, -hh, hl],   // 3: bottom left
        [hw, -hh, hl],    // 4: bottom right
        [0.0, hh, hl],    // 5: top center
        // Left slope
        [-hw, -hh, -hl],  // 6
        [-hw, -hh, hl],   // 7
        [0.0, hh, hl],    // 8
        [0.0, hh, -hl],   // 9
        // Right slope
        [hw, -hh, -hl],   // 10
        [hw, -hh, hl],    // 11
        [0.0, hh, hl],    // 12
        [0.0, hh, -hl],   // 13
    ];

    // Calculate normals
    let front_normal = [0.0, 0.0, -1.0];
    let back_normal = [0.0, 0.0, 1.0];

    // Left slope normal (pointing up-left)
    let left_slope_angle = (height / hw).atan();
    let left_normal = [
        -left_slope_angle.cos(),
        left_slope_angle.sin(),
        0.0,
    ];

    // Right slope normal (pointing up-right)
    let right_normal = [
        left_slope_angle.cos(),
        left_slope_angle.sin(),
        0.0,
    ];

    let normals = vec![
        front_normal, front_normal, front_normal,
        back_normal, back_normal, back_normal,
        left_normal, left_normal, left_normal, left_normal,
        right_normal, right_normal, right_normal, right_normal,
    ];

    let uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [0.5, 1.0],
        [0.0, 0.0], [1.0, 0.0], [0.5, 1.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];

    let indices = vec![
        // Front triangle
        0, 2, 1,
        // Back triangle
        3, 4, 5,
        // Left slope
        6, 8, 7,
        6, 9, 8,
        // Right slope
        10, 11, 12,
        10, 12, 13,
    ];

    Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
}

//! Traffic light generation at intersections.

use bevy::prelude::*;
use petgraph::graph::NodeIndex;

use crate::procgen::roads::RoadGraph;
use crate::render::road_mesh::RoadMeshGenerated;

pub struct TrafficLightsPlugin;

/// Traffic light phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightPhase {
    #[default]
    Green,
    Yellow,
    Red,
}

/// Controller for a traffic light at an intersection.
/// One controller per intersection manages the light cycling.
#[derive(Component)]
pub struct TrafficLightController {
    pub phase: LightPhase,
    pub timer: f32,
    pub node_index: NodeIndex,
    pub green_duration: f32,
    pub yellow_duration: f32,
    pub red_duration: f32,
}

impl Plugin for TrafficLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrafficLightConfig>()
            .add_systems(Update, (
                spawn_traffic_lights.run_if(should_spawn_lights),
                update_traffic_light_phases,
            ));
    }
}

fn should_spawn_lights(
    road_mesh_query: Query<&RoadMeshGenerated>,
    light_query: Query<&TrafficLight>,
) -> bool {
    !road_mesh_query.is_empty() && light_query.is_empty()
}

#[derive(Component)]
pub struct TrafficLight;

#[derive(Resource)]
pub struct TrafficLightConfig {
    pub pole_height: f32,
    pub pole_radius: f32,
    pub box_width: f32,
    pub box_height: f32,
    pub box_depth: f32,
    pub light_radius: f32,
    pub offset_from_center: f32,
}

impl Default for TrafficLightConfig {
    fn default() -> Self {
        Self {
            pole_height: 5.5,     // ~5.5m pole (realistic)
            pole_radius: 0.12,
            box_width: 0.6,
            box_height: 1.4,      // Traffic signal housing
            box_depth: 0.5,
            light_radius: 0.15,
            offset_from_center: 7.0,
        }
    }
}

fn spawn_traffic_lights(
    mut commands: Commands,
    road_graph: Res<RoadGraph>,
    config: Res<TrafficLightConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning traffic lights...");

    // Materials
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.5,
        metallic: 0.6,
        ..default()
    });

    let box_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        perceptual_roughness: 0.7,
        metallic: 0.3,
        ..default()
    });

    // Light materials (with emissive for glow effect)
    let red_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        emissive: LinearRgba::new(1.0, 0.1, 0.1, 1.0),
        ..default()
    });

    let yellow_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.1),
        emissive: LinearRgba::new(1.0, 0.85, 0.1, 1.0),
        ..default()
    });

    let green_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.8, 0.2),
        emissive: LinearRgba::new(0.1, 1.0, 0.2, 1.0),
        ..default()
    });

    // Meshes
    let pole_mesh = meshes.add(Cylinder::new(config.pole_radius, config.pole_height));
    let box_mesh = meshes.add(Cuboid::new(config.box_width, config.box_height, config.box_depth));
    let light_mesh = meshes.add(Sphere::new(config.light_radius));

    let mut light_count = 0;

    // Find intersections (nodes with 3+ connections)
    for (node_idx, node) in road_graph.nodes() {
        let neighbors: Vec<NodeIndex> = road_graph.graph.neighbors(node_idx).collect();

        if neighbors.len() < 3 {
            continue; // Not a real intersection
        }

        // Spawn a traffic light controller for this intersection
        commands.spawn(TrafficLightController {
            phase: LightPhase::Green,
            timer: 0.0,
            node_index: node_idx,
            green_duration: 12.0,
            yellow_duration: 3.0,
            red_duration: 12.0,
        });

        // Get directions to neighboring roads
        let mut road_directions: Vec<Vec2> = Vec::new();
        for neighbor_idx in &neighbors {
            if let Some(neighbor_node) = road_graph.graph.node_weight(*neighbor_idx) {
                let dir = (neighbor_node.position - node.position).normalize_or_zero();
                road_directions.push(dir);
            }
        }

        // Place traffic lights at corners of the intersection
        for (i, dir) in road_directions.iter().enumerate() {
            // Rotate 45 degrees to place at corner between roads
            let next_dir = road_directions[(i + 1) % road_directions.len()];
            let corner_dir = (*dir + next_dir).normalize_or_zero();

            if corner_dir.length_squared() < 0.01 {
                continue;
            }

            let light_pos = node.position + corner_dir * config.offset_from_center;

            // Calculate facing direction (toward intersection center)
            let facing = -corner_dir;
            let angle = facing.y.atan2(facing.x);

            // Spawn pole
            commands.spawn((
                Mesh3d(pole_mesh.clone()),
                MeshMaterial3d(pole_material.clone()),
                Transform::from_xyz(light_pos.x, config.pole_height / 2.0, light_pos.y),
                TrafficLight,
            ));

            // Spawn traffic light box
            let box_y = config.pole_height + config.box_height / 2.0;
            commands.spawn((
                Mesh3d(box_mesh.clone()),
                MeshMaterial3d(box_material.clone()),
                Transform::from_xyz(light_pos.x, box_y, light_pos.y)
                    .with_rotation(Quat::from_rotation_y(-angle)),
                TrafficLight,
            ));

            // Spawn the three lights (red, yellow, green from top to bottom)
            let light_spacing = config.box_height / 4.0;
            let light_offset = config.box_depth / 2.0 + config.light_radius * 0.5;

            // Calculate forward direction for light placement
            let forward = Vec2::new(facing.x, facing.y).normalize_or_zero();

            // Red light (top)
            let red_y = box_y + light_spacing;
            commands.spawn((
                Mesh3d(light_mesh.clone()),
                MeshMaterial3d(red_light.clone()),
                Transform::from_xyz(
                    light_pos.x + forward.x * light_offset,
                    red_y,
                    light_pos.y + forward.y * light_offset,
                ),
                TrafficLight,
            ));

            // Yellow light (middle)
            commands.spawn((
                Mesh3d(light_mesh.clone()),
                MeshMaterial3d(yellow_light.clone()),
                Transform::from_xyz(
                    light_pos.x + forward.x * light_offset,
                    box_y,
                    light_pos.y + forward.y * light_offset,
                ),
                TrafficLight,
            ));

            // Green light (bottom)
            let green_y = box_y - light_spacing;
            commands.spawn((
                Mesh3d(light_mesh.clone()),
                MeshMaterial3d(green_light.clone()),
                Transform::from_xyz(
                    light_pos.x + forward.x * light_offset,
                    green_y,
                    light_pos.y + forward.y * light_offset,
                ),
                TrafficLight,
            ));

            light_count += 1;
        }
    }

    info!("Spawned {} traffic lights", light_count);
}

/// Update traffic light phases based on timers.
fn update_traffic_light_phases(
    time: Res<Time>,
    mut controllers: Query<&mut TrafficLightController>,
) {
    let dt = time.delta_secs();

    for mut controller in controllers.iter_mut() {
        controller.timer += dt;

        // Check if it's time to transition to next phase
        let phase_duration = match controller.phase {
            LightPhase::Green => controller.green_duration,
            LightPhase::Yellow => controller.yellow_duration,
            LightPhase::Red => controller.red_duration,
        };

        if controller.timer >= phase_duration {
            controller.timer = 0.0;
            controller.phase = match controller.phase {
                LightPhase::Green => LightPhase::Yellow,
                LightPhase::Yellow => LightPhase::Red,
                LightPhase::Red => LightPhase::Green,
            };
        }
    }
}

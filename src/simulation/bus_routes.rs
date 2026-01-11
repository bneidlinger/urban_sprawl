//! Bus route system with buses following defined routes and stopping at bus stops.
//!
//! Creates bus routes along major roads and spawns buses that follow them,
//! stopping at bus stops to pick up passengers.

use bevy::prelude::*;
use petgraph::graph::EdgeIndex;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::roads::{RoadGraph, RoadType};
use crate::render::bus_stops::{BusStop, BusStopsSpawned};
use crate::render::road_mesh::RoadMeshGenerated;

pub struct BusRoutesPlugin;

impl Plugin for BusRoutesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BusRouteConfig>()
            .init_resource::<BusRoutes>()
            .init_resource::<BusesSpawned>()
            .init_resource::<RoutesGenerated>()
            .add_systems(
                Update,
                (
                    generate_bus_routes.run_if(should_generate_routes),
                    spawn_buses.run_if(should_spawn_buses),
                    update_bus_movement,
                    update_bus_transforms,
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct BusesSpawned(pub bool);

#[derive(Resource, Default)]
pub struct RoutesGenerated(pub bool);

fn should_generate_routes(
    road_mesh: Query<&RoadMeshGenerated>,
    bus_stops_spawned: Res<BusStopsSpawned>,
    routes_generated: Res<RoutesGenerated>,
) -> bool {
    !road_mesh.is_empty() && bus_stops_spawned.0 && !routes_generated.0
}

fn should_spawn_buses(routes: Res<BusRoutes>, spawned: Res<BusesSpawned>) -> bool {
    !routes.routes.is_empty() && !spawned.0
}

#[derive(Resource)]
pub struct BusRouteConfig {
    pub seed: u64,
    pub max_routes: usize,
    pub buses_per_route: usize,
    pub bus_speed: f32,
    pub stop_duration: f32,
    pub bus_length: f32,
    pub bus_width: f32,
    pub bus_height: f32,
}

impl Default for BusRouteConfig {
    fn default() -> Self {
        Self {
            seed: 99999,
            max_routes: 4,
            buses_per_route: 2,
            bus_speed: 8.0,
            stop_duration: 4.0,
            bus_length: 10.0,
            bus_width: 2.5,
            bus_height: 3.0,
        }
    }
}

/// A bus route with waypoints and stop indices.
#[derive(Clone)]
pub struct BusRoute {
    /// Edges that make up this route.
    pub edges: Vec<EdgeIndex>,
    /// Indices into edges where bus stops are located.
    pub stop_indices: Vec<usize>,
    /// Route color for identification.
    pub color: Color,
}

/// Resource holding all bus routes.
#[derive(Resource, Default)]
pub struct BusRoutes {
    pub routes: Vec<BusRoute>,
}

/// Bus vehicle component.
#[derive(Component)]
pub struct Bus {
    pub route_index: usize,
    /// Current edge index in route.
    pub edge_index: usize,
    /// Progress along current edge (0.0 to 1.0).
    pub progress: f32,
    /// Current speed.
    pub speed: f32,
    /// True if stopped at a bus stop.
    pub at_stop: bool,
    /// Timer for stop duration.
    pub stop_timer: f32,
    /// Direction (1 = forward through route, -1 = reverse).
    pub direction: f32,
}

/// Route colors for different bus lines.
const ROUTE_COLORS: &[Color] = &[
    Color::srgb(0.2, 0.5, 0.8),  // Blue
    Color::srgb(0.8, 0.3, 0.2),  // Red
    Color::srgb(0.2, 0.7, 0.3),  // Green
    Color::srgb(0.7, 0.5, 0.2),  // Orange
];

fn generate_bus_routes(
    road_graph: Res<RoadGraph>,
    bus_stops: Query<(&BusStop, &Transform)>,
    config: Res<BusRouteConfig>,
    mut routes: ResMut<BusRoutes>,
    mut routes_generated: ResMut<RoutesGenerated>,
) {
    routes_generated.0 = true;
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Collect bus stop positions
    let stop_positions: Vec<Vec2> = bus_stops
        .iter()
        .map(|(_, t)| Vec2::new(t.translation.x, t.translation.z))
        .collect();

    if stop_positions.len() < 2 {
        info!("Not enough bus stops for routes");
        return;
    }

    // Collect major road edges
    let major_edges: Vec<EdgeIndex> = road_graph
        .edges()
        .enumerate()
        .filter_map(|(i, e)| {
            if matches!(e.road_type, RoadType::Major | RoadType::Highway) {
                Some(EdgeIndex::new(i))
            } else {
                None
            }
        })
        .collect();

    if major_edges.is_empty() {
        info!("No major roads for bus routes");
        return;
    }

    // Generate routes by connecting sequences of edges
    for route_idx in 0..config.max_routes.min(major_edges.len() / 3) {
        let start_idx = rng.gen_range(0..major_edges.len());
        let mut route_edges: Vec<EdgeIndex> = Vec::new();
        let mut current_edge = major_edges[start_idx];
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();

        // Build route by following connected edges
        for _ in 0..15 {
            // Max 15 edges per route
            route_edges.push(current_edge);
            visited.insert(current_edge.index());

            // Find next connected edge
            if let Some((_, node_b)) = road_graph.edge_endpoints(current_edge) {
                let next_edges: Vec<EdgeIndex> = road_graph
                    .edges_of_node(node_b)
                    .filter(|&e| {
                        !visited.contains(&e.index())
                            && major_edges.contains(&e)
                    })
                    .collect();

                if next_edges.is_empty() {
                    break;
                }
                current_edge = next_edges[rng.gen_range(0..next_edges.len())];
            } else {
                break;
            }
        }

        if route_edges.len() < 3 {
            continue;
        }

        // Find which edges have bus stops nearby
        let mut stop_indices: Vec<usize> = Vec::new();
        for (edge_idx, &edge) in route_edges.iter().enumerate() {
            if let Some(edge_data) = road_graph.edge_by_index(edge) {
                let edge_center = edge_data
                    .points
                    .iter()
                    .fold(Vec2::ZERO, |acc, p| acc + *p)
                    / edge_data.points.len().max(1) as f32;

                // Check if any bus stop is near this edge
                let has_stop = stop_positions
                    .iter()
                    .any(|&pos| pos.distance(edge_center) < 20.0);

                if has_stop {
                    stop_indices.push(edge_idx);
                }
            }
        }

        routes.routes.push(BusRoute {
            edges: route_edges,
            stop_indices,
            color: ROUTE_COLORS[route_idx % ROUTE_COLORS.len()],
        });
    }

    info!("Generated {} bus routes", routes.routes.len());
}

fn spawn_buses(
    mut commands: Commands,
    config: Res<BusRouteConfig>,
    routes: Res<BusRoutes>,
    road_graph: Res<RoadGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<BusesSpawned>,
) {
    spawned.0 = true;

    if routes.routes.is_empty() {
        return;
    }

    let mut rng = StdRng::seed_from_u64(config.seed + 1);

    // Meshes
    let body_mesh = meshes.add(Cuboid::new(
        config.bus_length,
        config.bus_height * 0.7,
        config.bus_width,
    ));
    let roof_mesh = meshes.add(Cuboid::new(
        config.bus_length - 0.5,
        config.bus_height * 0.15,
        config.bus_width - 0.2,
    ));
    let window_mesh = meshes.add(Cuboid::new(1.5, config.bus_height * 0.25, 0.05));

    let window_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.3, 0.4, 0.7),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.88),
        perceptual_roughness: 0.6,
        ..default()
    });

    for (route_idx, route) in routes.routes.iter().enumerate() {
        let body_material = materials.add(StandardMaterial {
            base_color: route.color,
            perceptual_roughness: 0.5,
            metallic: 0.3,
            ..default()
        });

        // Spawn buses evenly distributed along route
        for bus_idx in 0..config.buses_per_route {
            let start_edge_idx = (bus_idx * route.edges.len() / config.buses_per_route) % route.edges.len();
            let edge = route.edges[start_edge_idx];

            // Get initial position
            let (pos, dir) = if let Some(edge_data) = road_graph.edge_by_index(edge) {
                let mid = edge_data.points.len() / 2;
                if mid < edge_data.points.len() {
                    let p = edge_data.points[mid];
                    let d = if mid + 1 < edge_data.points.len() {
                        (edge_data.points[mid + 1] - p).normalize_or_zero()
                    } else if mid > 0 {
                        (p - edge_data.points[mid - 1]).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    (p, d)
                } else {
                    (Vec2::ZERO, Vec2::X)
                }
            } else {
                continue;
            };

            let angle = dir.y.atan2(dir.x);
            let direction = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

            commands
                .spawn((
                    Transform::from_xyz(pos.x, config.bus_height / 2.0 + 0.3, pos.y)
                        .with_rotation(Quat::from_rotation_y(-angle + PI / 2.0)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Bus {
                        route_index: route_idx,
                        edge_index: start_edge_idx,
                        progress: 0.5,
                        speed: config.bus_speed,
                        at_stop: false,
                        stop_timer: 0.0,
                        direction,
                    },
                ))
                .with_children(|parent| {
                    // Body
                    parent.spawn((
                        Mesh3d(body_mesh.clone()),
                        MeshMaterial3d(body_material.clone()),
                        Transform::IDENTITY,
                    ));

                    // Roof
                    parent.spawn((
                        Mesh3d(roof_mesh.clone()),
                        MeshMaterial3d(roof_material.clone()),
                        Transform::from_xyz(0.0, config.bus_height * 0.4, 0.0),
                    ));

                    // Windows
                    let window_count = 4;
                    for w in 0..window_count {
                        let x = (w as f32 - (window_count - 1) as f32 / 2.0) * 2.0;
                        // Both sides
                        for z in [-config.bus_width / 2.0 - 0.03, config.bus_width / 2.0 + 0.03] {
                            parent.spawn((
                                Mesh3d(window_mesh.clone()),
                                MeshMaterial3d(window_material.clone()),
                                Transform::from_xyz(x, config.bus_height * 0.1, z),
                            ));
                        }
                    }
                });
        }
    }

    info!(
        "Spawned {} buses across {} routes",
        routes.routes.len() * config.buses_per_route,
        routes.routes.len()
    );
}

fn update_bus_movement(
    time: Res<Time>,
    config: Res<BusRouteConfig>,
    routes: Res<BusRoutes>,
    road_graph: Res<RoadGraph>,
    mut buses: Query<&mut Bus>,
) {
    let dt = time.delta_secs();

    for mut bus in buses.iter_mut() {
        // Handle stop timer
        if bus.at_stop {
            bus.stop_timer -= dt;
            if bus.stop_timer <= 0.0 {
                bus.at_stop = false;
            }
            continue;
        }

        let Some(route) = routes.routes.get(bus.route_index) else {
            continue;
        };
        let Some(&edge) = route.edges.get(bus.edge_index) else {
            continue;
        };
        let Some(edge_data) = road_graph.edge_by_index(edge) else {
            continue;
        };

        let edge_length = edge_data.length.max(1.0);
        let progress_delta = (bus.speed * dt) / edge_length;

        bus.progress += progress_delta * bus.direction;

        // Check for edge transition
        if bus.progress >= 1.0 {
            // Check if this edge has a bus stop
            if route.stop_indices.contains(&bus.edge_index) {
                bus.at_stop = true;
                bus.stop_timer = config.stop_duration;
                bus.progress = 1.0;
            }

            // Move to next edge
            if bus.direction > 0.0 {
                bus.edge_index = (bus.edge_index + 1) % route.edges.len();
            } else {
                bus.edge_index = if bus.edge_index == 0 {
                    route.edges.len() - 1
                } else {
                    bus.edge_index - 1
                };
            }
            bus.progress = 0.0;

            // Reverse at ends
            if bus.edge_index == 0 || bus.edge_index == route.edges.len() - 1 {
                bus.direction = -bus.direction;
            }
        } else if bus.progress < 0.0 {
            // Moving backwards
            if route.stop_indices.contains(&bus.edge_index) {
                bus.at_stop = true;
                bus.stop_timer = config.stop_duration;
                bus.progress = 0.0;
            }

            if bus.direction > 0.0 {
                bus.edge_index = (bus.edge_index + 1) % route.edges.len();
            } else {
                bus.edge_index = if bus.edge_index == 0 {
                    route.edges.len() - 1
                } else {
                    bus.edge_index - 1
                };
            }
            bus.progress = 1.0;

            if bus.edge_index == 0 || bus.edge_index == route.edges.len() - 1 {
                bus.direction = -bus.direction;
            }
        }
    }
}

fn update_bus_transforms(
    config: Res<BusRouteConfig>,
    routes: Res<BusRoutes>,
    road_graph: Res<RoadGraph>,
    mut buses: Query<(&Bus, &mut Transform)>,
) {
    for (bus, mut transform) in buses.iter_mut() {
        let Some(route) = routes.routes.get(bus.route_index) else {
            continue;
        };
        let Some(&edge) = route.edges.get(bus.edge_index) else {
            continue;
        };
        let Some(edge_data) = road_graph.edge_by_index(edge) else {
            continue;
        };

        let (pos, dir) = interpolate_edge(edge_data.points.as_slice(), bus.progress);

        // Offset to right side of road
        let perp = Vec2::new(-dir.y, dir.x);
        let offset_pos = pos + perp * 2.0 * bus.direction.signum();

        transform.translation.x = offset_pos.x;
        transform.translation.z = offset_pos.y;
        transform.translation.y = config.bus_height / 2.0 + 0.3;

        // Face direction of travel
        let facing = if bus.direction > 0.0 { dir } else { -dir };
        let angle = facing.y.atan2(facing.x);
        transform.rotation = Quat::from_rotation_y(-angle + PI / 2.0);
    }
}

fn interpolate_edge(points: &[Vec2], progress: f32) -> (Vec2, Vec2) {
    if points.is_empty() {
        return (Vec2::ZERO, Vec2::X);
    }
    if points.len() == 1 {
        return (points[0], Vec2::X);
    }

    let total_length: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    if total_length <= 0.0 {
        return (points[0], Vec2::X);
    }

    let target_dist = progress.clamp(0.0, 1.0) * total_length;
    let mut accumulated = 0.0;

    for window in points.windows(2) {
        let seg_len = window[0].distance(window[1]);
        if accumulated + seg_len >= target_dist {
            let local_t = if seg_len > 0.0 {
                (target_dist - accumulated) / seg_len
            } else {
                0.0
            };
            let pos = window[0].lerp(window[1], local_t);
            let dir = (window[1] - window[0]).normalize_or_zero();
            return (pos, dir);
        }
        accumulated += seg_len;
    }

    let last = *points.last().unwrap();
    let dir = if points.len() >= 2 {
        (points[points.len() - 1] - points[points.len() - 2]).normalize_or_zero()
    } else {
        Vec2::X
    };
    (last, dir)
}

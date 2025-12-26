//! Rendering systems for hardware instancing and GPU culling.

use bevy::prelude::*;

pub mod building_spawner;
pub mod instancing;
pub mod road_markings;
pub mod road_mesh;
pub mod street_lamps;
pub mod traffic_lights;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(instancing::InstancingPlugin)
            .add_plugins(road_mesh::RoadMeshPlugin)
            .add_plugins(road_markings::RoadMarkingsPlugin)
            .add_plugins(building_spawner::BuildingSpawnerPlugin)
            .add_plugins(street_lamps::StreetLampsPlugin)
            .add_plugins(traffic_lights::TrafficLightsPlugin);
    }
}

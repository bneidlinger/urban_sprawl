//! Rendering systems for hardware instancing and GPU culling.

use bevy::prelude::*;

pub mod building_shadows;
pub mod building_spawner;
pub mod cloud_shadows;
pub mod crosswalks;
pub mod day_night;
pub mod instancing;
pub mod parked_cars;
pub mod road_markings;
pub mod road_mesh;
pub mod street_furniture;
pub mod street_lamps;
pub mod traffic_lights;
pub mod window_lights;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(day_night::DayNightPlugin)
            .add_plugins(instancing::InstancingPlugin)
            .add_plugins(road_mesh::RoadMeshPlugin)
            .add_plugins(road_markings::RoadMarkingsPlugin)
            .add_plugins(building_spawner::BuildingSpawnerPlugin)
            .add_plugins(building_shadows::BuildingShadowsPlugin)
            .add_plugins(street_lamps::StreetLampsPlugin)
            .add_plugins(traffic_lights::TrafficLightsPlugin)
            .add_plugins(crosswalks::CrosswalksPlugin)
            .add_plugins(parked_cars::ParkedCarsPlugin)
            .add_plugins(street_furniture::StreetFurniturePlugin)
            .add_plugins(window_lights::WindowLightsPlugin)
            .add_plugins(cloud_shadows::CloudShadowsPlugin);
    }
}

//! Rendering systems for hardware instancing and GPU culling.

use bevy::prelude::*;

pub mod bridges;
pub mod building_instances;
pub mod building_shadows;
pub mod building_spawner;
pub mod cloud_shadows;
pub mod clustered_shading;
pub mod crosswalks;
pub mod day_night;
pub mod entrance_lights;
pub mod facade_textures;
pub mod gpu_culling;
pub mod hzb;
pub mod instancing;
pub mod mesh_pools;
pub mod neon_signs;
pub mod parked_cars;
pub mod road_markings;
pub mod road_mesh;
pub mod rooftop_details;
pub mod street_details;
pub mod street_furniture;
pub mod street_lamps;
pub mod street_trees;
pub mod tilt_shift;
pub mod traffic_lights;
pub mod vehicle_lights;
pub mod water;
pub mod weather;
pub mod window_instances;
pub mod window_lights;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // Core infrastructure plugins (order matters)
        app.add_plugins(mesh_pools::MeshPoolsPlugin)
            .add_plugins(building_instances::BuildingInstancesPlugin)
            .add_plugins(facade_textures::FacadeTexturesPlugin)
            .add_plugins(gpu_culling::GpuCullingPlugin)
            .add_plugins(hzb::HzbPlugin)
            .add_plugins(day_night::DayNightPlugin)
            .add_plugins(instancing::InstancingPlugin)
            .add_plugins(road_mesh::RoadMeshPlugin)
            .add_plugins(road_markings::RoadMarkingsPlugin)
            .add_plugins(bridges::BridgesPlugin)
            .add_plugins(building_spawner::BuildingSpawnerPlugin)
            .add_plugins(building_shadows::BuildingShadowsPlugin)
            .add_plugins(street_lamps::StreetLampsPlugin)
            .add_plugins(traffic_lights::TrafficLightsPlugin)
            .add_plugins(crosswalks::CrosswalksPlugin)
            .add_plugins(parked_cars::ParkedCarsPlugin)
            .add_plugins(street_furniture::StreetFurniturePlugin)
            .add_plugins(street_details::StreetDetailsPlugin)
            .add_plugins(street_trees::StreetTreesPlugin)
            .add_plugins(window_instances::WindowInstancesPlugin)
            .add_plugins(window_lights::WindowLightsPlugin)
            .add_plugins(rooftop_details::RooftopDetailsPlugin)
            .add_plugins(neon_signs::NeonSignsPlugin)
            .add_plugins(entrance_lights::EntranceLightsPlugin)
            .add_plugins(vehicle_lights::VehicleLightsPlugin)
            .add_plugins(cloud_shadows::CloudShadowsPlugin)
            .add_plugins(water::WaterPlugin)
            .add_plugins(weather::WeatherPlugin)
            .add_plugins(tilt_shift::TiltShiftPlugin)
            .add_plugins(clustered_shading::ClusteredShadingPlugin);
    }
}

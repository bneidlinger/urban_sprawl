//! Procedural generation systems.
//!
//! - Tensor fields for road networks
//! - OBB subdivision for parcels
//! - Shape grammars for buildings
//! - Wave Function Collapse for zoning
//! - River generation

use bevy::prelude::*;

pub mod block_extractor;
pub mod building_factory;
pub mod buildings;
pub mod lot_engine;
pub mod lot_geometry;
pub mod parcels;
pub mod river;
pub mod road_generator;
pub mod roads;
pub mod streamline;
pub mod tensor;
pub mod zoning;

pub struct ProcgenPlugin;

impl Plugin for ProcgenPlugin {
    fn build(&self, app: &mut App) {
        // River must be generated before roads so roads can avoid/bridge water
        app.add_plugins(river::RiverPlugin)
            .add_plugins(tensor::TensorFieldPlugin)
            .add_plugins(roads::RoadsPlugin)
            .add_plugins(road_generator::RoadGeneratorPlugin)
            .add_plugins(block_extractor::BlockExtractorPlugin)
            .add_plugins(lot_engine::LotEnginePlugin)
            .add_plugins(building_factory::BuildingFactoryPlugin);
    }
}

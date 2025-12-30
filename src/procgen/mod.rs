//! Procedural generation systems.
//!
//! - Tensor fields for road networks
//! - OBB subdivision for parcels
//! - Shape grammars for buildings
//! - Wave Function Collapse for zoning

use bevy::prelude::*;

pub mod block_extractor;
pub mod buildings;
pub mod lot_engine;
pub mod parcels;
pub mod road_generator;
pub mod roads;
pub mod streamline;
pub mod tensor;
pub mod zoning;

pub struct ProcgenPlugin;

impl Plugin for ProcgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tensor::TensorFieldPlugin)
            .add_plugins(roads::RoadsPlugin)
            .add_plugins(road_generator::RoadGeneratorPlugin)
            .add_plugins(block_extractor::BlockExtractorPlugin)
            .add_plugins(lot_engine::LotEnginePlugin);
    }
}

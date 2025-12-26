//! Road graph construction from streamlines.
//!
//! Uses petgraph for the underlying graph structure.

use bevy::prelude::*;
use petgraph::graph::{NodeIndex, UnGraph};
use smallvec::SmallVec;

pub struct RoadsPlugin;

impl Plugin for RoadsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadGraph>();
    }
}

/// A node in the road network (intersection or endpoint).
#[derive(Clone, Debug)]
pub struct RoadNode {
    pub position: Vec2,
    pub node_type: RoadNodeType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoadNodeType {
    Intersection,
    Endpoint,
    DeadEnd,
}

/// An edge in the road network (road segment).
#[derive(Clone, Debug)]
pub struct RoadEdge {
    /// Intermediate points along the road (for curved roads).
    pub points: SmallVec<[Vec2; 8]>,
    /// Road classification.
    pub road_type: RoadType,
    /// Length in world units.
    pub length: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoadType {
    Highway,
    Major,
    Minor,
    Alley,
}

impl RoadEdge {
    pub fn new(points: SmallVec<[Vec2; 8]>, road_type: RoadType) -> Self {
        let length = Self::calculate_length(&points);
        Self {
            points,
            road_type,
            length,
        }
    }

    fn calculate_length(points: &[Vec2]) -> f32 {
        points
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum()
    }
}

/// The road network graph resource.
#[derive(Resource, Default)]
pub struct RoadGraph {
    pub graph: UnGraph<RoadNode, RoadEdge>,
    /// Spatial index for fast nearest-node queries.
    node_positions: Vec<(NodeIndex, Vec2)>,
}

impl RoadGraph {
    /// Add a node to the graph.
    pub fn add_node(&mut self, position: Vec2, node_type: RoadNodeType) -> NodeIndex {
        let node = RoadNode { position, node_type };
        let idx = self.graph.add_node(node);
        self.node_positions.push((idx, position));
        idx
    }

    /// Add an edge between two nodes.
    pub fn add_edge(
        &mut self,
        a: NodeIndex,
        b: NodeIndex,
        points: SmallVec<[Vec2; 8]>,
        road_type: RoadType,
    ) {
        let edge = RoadEdge::new(points, road_type);
        self.graph.add_edge(a, b, edge);
    }

    /// Find the nearest node within a radius.
    pub fn find_nearest(&self, position: Vec2, max_distance: f32) -> Option<NodeIndex> {
        let mut best: Option<(NodeIndex, f32)> = None;

        for &(idx, pos) in &self.node_positions {
            let dist = position.distance(pos);
            if dist <= max_distance {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((idx, dist));
                }
            }
        }

        best.map(|(idx, _)| idx)
    }

    /// Try to snap a position to an existing node, or create a new one.
    pub fn snap_or_create(
        &mut self,
        position: Vec2,
        snap_distance: f32,
        node_type: RoadNodeType,
    ) -> NodeIndex {
        if let Some(existing) = self.find_nearest(position, snap_distance) {
            // Upgrade endpoint to intersection if needed
            if let Some(node) = self.graph.node_weight_mut(existing) {
                if node.node_type == RoadNodeType::Endpoint {
                    node.node_type = RoadNodeType::Intersection;
                }
            }
            existing
        } else {
            self.add_node(position, node_type)
        }
    }

    /// Get all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeIndex, &RoadNode)> {
        self.graph.node_indices().map(|i| (i, &self.graph[i]))
    }

    /// Get all edges.
    pub fn edges(&self) -> impl Iterator<Item = &RoadEdge> {
        self.graph.edge_weights()
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get edge count.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

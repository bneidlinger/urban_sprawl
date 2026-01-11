//! Road graph construction from streamlines.
//!
//! Uses petgraph for the underlying graph structure.

#![allow(dead_code)]

use bevy::prelude::*;
use petgraph::graph::{EdgeIndex, NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
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
    /// Whether this road segment crosses water (needs bridge).
    pub crosses_water: bool,
    /// Entry point where road enters water (if crosses_water).
    pub water_entry: Option<Vec2>,
    /// Exit point where road exits water (if crosses_water).
    pub water_exit: Option<Vec2>,
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
            crosses_water: false,
            water_entry: None,
            water_exit: None,
        }
    }

    /// Create a road edge that crosses water.
    pub fn new_bridge(
        points: SmallVec<[Vec2; 8]>,
        road_type: RoadType,
        water_entry: Vec2,
        water_exit: Vec2,
    ) -> Self {
        let length = Self::calculate_length(&points);
        Self {
            points,
            road_type,
            length,
            crosses_water: true,
            water_entry: Some(water_entry),
            water_exit: Some(water_exit),
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

    /// Add a bridge edge between two nodes (crosses water).
    pub fn add_bridge_edge(
        &mut self,
        a: NodeIndex,
        b: NodeIndex,
        points: SmallVec<[Vec2; 8]>,
        road_type: RoadType,
        water_entry: Vec2,
        water_exit: Vec2,
    ) {
        let edge = RoadEdge::new_bridge(points, road_type, water_entry, water_exit);
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

    /// Get all edge indices.
    pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.graph.edge_indices()
    }

    /// Get an edge by its index.
    pub fn edge_by_index(&self, idx: EdgeIndex) -> Option<&RoadEdge> {
        self.graph.edge_weight(idx)
    }

    /// Get the endpoint node indices for an edge.
    pub fn edge_endpoints(&self, idx: EdgeIndex) -> Option<(NodeIndex, NodeIndex)> {
        self.graph.edge_endpoints(idx)
    }

    /// Get a node by its index.
    pub fn node_by_index(&self, idx: NodeIndex) -> Option<&RoadNode> {
        self.graph.node_weight(idx)
    }

    /// Get neighbor node indices for a given node.
    pub fn neighbors(&self, idx: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors(idx)
    }

    /// Get edges connected to a node with their indices.
    pub fn edges_of_node(&self, idx: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.graph.edges(idx).map(|e| e.id())
    }

    /// Remove an edge by its index. Returns the edge data if it existed.
    pub fn remove_edge(&mut self, idx: EdgeIndex) -> Option<RoadEdge> {
        self.graph.remove_edge(idx)
    }

    /// Remove a node by its index. Returns the node data if it existed.
    /// Note: This also removes all edges connected to this node.
    pub fn remove_node(&mut self, idx: NodeIndex) -> Option<RoadNode> {
        // Remove from spatial index
        self.node_positions.retain(|(node_idx, _)| *node_idx != idx);
        self.graph.remove_node(idx)
    }

    /// Find the edge index between two nodes.
    pub fn find_edge(&self, a: NodeIndex, b: NodeIndex) -> Option<EdgeIndex> {
        self.graph.find_edge(a, b)
    }

    /// Check if a node has any connected edges.
    pub fn node_has_edges(&self, idx: NodeIndex) -> bool {
        self.graph.edges(idx).next().is_some()
    }

    /// Get the degree (number of connected edges) of a node.
    pub fn node_degree(&self, idx: NodeIndex) -> usize {
        self.graph.edges(idx).count()
    }

    /// Re-add a node at a specific index (for undo). Returns the new NodeIndex.
    /// Note: The index may differ from the original if graph was modified.
    pub fn restore_node(&mut self, position: Vec2, node_type: RoadNodeType) -> NodeIndex {
        self.add_node(position, node_type)
    }

    /// Add an edge with full RoadEdge data (for undo/redo).
    pub fn add_edge_data(&mut self, a: NodeIndex, b: NodeIndex, edge: RoadEdge) -> EdgeIndex {
        self.graph.add_edge(a, b, edge)
    }
}

// src/engine/dag.rs

/// Compile and validate declarative mission into actionable DiGraph

use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use crate::core::board::{Mission, MissionNode, MissionEdge};

pub struct MissionDag {
    pub graph: DiGraph<MissionNode, MissionEdge>,
    pub index_map: HashMap<String, NodeIndex>,
}

impl MissionDag {
    pub fn try_from_mission(mission: &Mission) -> Result<Self, String> {
        let mut graph = DiGraph::new();
        let mut index_map = HashMap::new();

        // 1. Add nodes to petgraph
        for node in &mission.nodes {
            let idx = graph.add_node(node.clone());
            index_map.insert(node.id.clone(), idx);
        }

        // 2. Add edges
        for edge in &mission.edges {
            let from_idx = index_map.get(&edge.from)
                .ok_or_else(|| format!("Cannot find node source: {}", edge.from))?;
            let to_idx = index_map.get(&edge.to)
                .ok_or_else(|| format!("Cannot find node target: {}", edge.to))?;

            // Directional edge from parent node to child node
            graph.add_edge(*from_idx, *to_idx, edge.clone());
        }

        // 3. Detect cycles (mission DAG should be Acyclic)
        if petgraph::algo::is_cyclic_directed(&graph) {
            return Err("Mission includes an invalid circular dependency!".into());
        }

        Ok(Self { graph, index_map })
    }
}
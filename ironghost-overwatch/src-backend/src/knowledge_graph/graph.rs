// src/knowledge_graph/graph.rs

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
}

// ----------
// nodes
// ----------

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeNodeKind {
    Concept,
    Claim,
    Evidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeNode {
    pub id      : String,
    pub kind    : KnowledgeNodeKind,
    pub label   : String,
    pub status  : Option<String>,
}

// ----------
// edges
// ----------

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEdgeKind {
    RelatedTo,  // Card     -> Card
    ClaimOf,    // Claim    -> Card
    Supports,   // Evidence -> Claim
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeEdge {
    pub from    : String,
    pub to      : String,
    pub relation: KnowledgeEdgeKind,
}
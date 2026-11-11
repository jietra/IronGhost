// src/knowledge_graph/builder.rs

use std::collections::HashMap;

use crate::knowledge::{
    mission_knowledge::MissionKnowledge,
    concept::extract_concepts,
};
use crate::knowledge_graph::graph::{
    KnowledgeEdge,
    KnowledgeEdgeKind,
    KnowledgeGraph,
    KnowledgeNode,
    KnowledgeNodeKind,
};

pub struct KnowledgeGraphBuilder;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EdgeKey {
    from    : String,
    to      : String,
    relation: KnowledgeEdgeKind,
}

impl KnowledgeGraphBuilder {

    pub fn build(knowledge: &MissionKnowledge) -> KnowledgeGraph {

        let mut nodes: HashMap<String, KnowledgeNode>  = HashMap::new();
        let mut edges: HashMap<EdgeKey, KnowledgeEdge> = HashMap::new();

        for card in knowledge.cards.values() {

            // =================================
            // 1. * Card: concept/card node
            // =================================

            let card_node_id: String = concept_node_id(&card.id);

            nodes.insert(
                card_node_id.clone(),
                KnowledgeNode {
                    id      : card_node_id.clone(),
                    kind    : KnowledgeNodeKind::Concept,
                    label   : card.title.clone(),
                    status  : Some(format!("{:?}", card.status)),
                },
            );

            // --------------------------------------------------------------------------
            // Edges: concept -> concept card (RelatedTo: relationships found in summary)
            // --------------------------------------------------------------------------

            Self::add_concept_refs(
                &mut nodes,
                &mut edges,
                &card_node_id,      // concept
                &card.id,       // card
                &card.summary,
            );

            for (claim_index, claim) in card.claims.iter().enumerate() {

                // =================================
                // 2. * Claim: claim node
                // =================================

                let claim_node_id: String = claim_node_id(&card.id, claim_index);

                nodes.insert(
                    claim_node_id.clone(),
                    KnowledgeNode {
                        id      : claim_node_id.clone(),
                        kind    : KnowledgeNodeKind::Claim,
                        label   : claim.statement.clone(),
                        status  : Some(format!("{:?}", claim.status)),
                    },
                );

                // ---------------------------------------------------------------
                // Edge: claim -> concept card (ClaimOF: claim belongs to concept)
                // ---------------------------------------------------------------

                Self::insert_edge(
                    &mut edges,
                    KnowledgeEdge {
                        from    : claim_node_id.clone(),
                        to      : card_node_id.clone(),
                        relation: KnowledgeEdgeKind::ClaimOf,
                    },
                );

                for evidence_id in &claim.evidence_ids {

                    // =================================
                    // 3. * Evidence: evidence node
                    // =================================

                    let evidence_node_id: String = evidence_node_id(evidence_id);

                    nodes
                        .entry(evidence_node_id.clone())
                        .or_insert_with(|| {
                            KnowledgeNode {
                                id      : evidence_node_id.clone(),
                                kind    : KnowledgeNodeKind::Evidence,
                                label   : evidence_id.clone(),
                                status  : None,
                            }
                        });
                    
                    // -----------------------------------------------------------
                    // Edge: evidence -> claim (Supports: evidence supports claim)
                    // -----------------------------------------------------------

                    Self::insert_edge(
                        &mut edges,
                        KnowledgeEdge {
                            // Reading direction: Evidence -(supports)-> Claim
                            from    : evidence_node_id,
                            to      : claim_node_id.clone(),
                            relation: KnowledgeEdgeKind::Supports,
                        },
                    );
                }

                // -------------------------------------
                // Cross-concept references inside claim
                // -------------------------------------

                Self::add_concept_refs(
                    &mut nodes,
                    &mut edges,
                    &claim_node_id,
                    &card.id,
                    &claim.statement,
                );

            }
        }
        
        // --------------------------------
        // Stable output ordering
        // --------------------------------

        let mut nodes: Vec<KnowledgeNode> = nodes.into_values().collect();
        let mut edges: Vec<KnowledgeEdge> = edges.into_values().collect();

        nodes.sort_by(
            |left, right|
                left.id.cmp(&right.id)
        );

        edges.sort_by(
            |left, right| {
                (
                    &left.from,
                    &left.to,
                    format!("{:?}", left.relation),
                )
                    .cmp(&(
                        &right.from,
                        &right.to,
                        format!("{:?}", right.relation),
                    ))
            }
        );

        KnowledgeGraph {
            nodes,
            edges,
        }

    }

    fn add_concept_refs(
        nodes               : &mut HashMap<String, KnowledgeNode>,
        edges               : &mut HashMap<EdgeKey,KnowledgeEdge>,
        source_node_id      : &str,
        source_concept_id   : &str,
        text                : &str,
    ) {
        for concept in extract_concepts(text) {

            // no self-referential edges.
            if concept.id == source_concept_id { continue; }

            let target_node_id: String = concept_node_id(&concept.id);

            nodes
                .entry(target_node_id.clone())
                .or_insert_with(|| {
                    KnowledgeNode {
                        id      : target_node_id.clone(),
                        kind    : KnowledgeNodeKind::Concept,
                        label   : concept.title.clone(),
                        status  : None,
                    }
                });

            Self::insert_edge(
                edges,
                KnowledgeEdge {
                    from    : source_node_id.to_string(),
                    to      : target_node_id,
                    relation: KnowledgeEdgeKind::RelatedTo,
                },
            );
        }
    }

    fn insert_edge(
        edges: &mut HashMap<EdgeKey, KnowledgeEdge>,
        edge : KnowledgeEdge,
    ) {
        let key: EdgeKey = EdgeKey {
            from    : edge.from.clone(),
            to      : edge.to.clone(),
            relation: edge.relation.clone(),
        };

        edges
            .entry(key)
            .or_insert(edge);
    }

}

// namespaces (helpers)

fn concept_node_id(concept_id: &str,) -> String {
    format!("concept:{concept_id}")
}

fn claim_node_id(card_id: &str, index: usize) -> String {
    format!("claim:{card_id}:{index}")
}

fn evidence_node_id(evidence_id: &str) -> String {
    format!("evidence:{evidence_id}")
}


#[cfg(test)]
mod tests {
    use super::*;

    use crate::knowledge::{
        card::{KnowledgeCard, CardStatus},
        claim::{KnowledgeClaim, ClaimStatus},
        mission_knowledge::MissionKnowledge,
    };

    fn sample_knowledge() -> MissionKnowledge {
        let mut knowledge: MissionKnowledge = MissionKnowledge::default();

        knowledge
            .upsert_card(KnowledgeCard {
                id      : "resource_exhaustion".into(),
                title   : "Resource Exhaustion".into(),
                status  : CardStatus::Investigating,
                summary : "Unbounded parsing may affect [[Protocol Parsing]].".into(),
                claims  : vec![
                    KnowledgeClaim {
                        statement   : "The connection buffer has no explicit size limit and contributes to [[Protocol Parsing]].".into(),
                        status      : ClaimStatus::Confirmed,
                        evidence_ids: vec!["mini-redis/src/connection.rs".into()],
                    },
                    KnowledgeClaim {
                        statement   : "Stored values have no explicit size limit.".into(),
                        status      : ClaimStatus::Supported,
                        evidence_ids: vec!["mini-redis/src/db.rs".into()],
                    },
                ],
            })
            .unwrap();

        knowledge
    }

    #[test]
    fn builds_card_claim_evidence_and_relation_graph() {
        let knowledge: MissionKnowledge = sample_knowledge();
        let graph    : KnowledgeGraph   = KnowledgeGraphBuilder::build(&knowledge);

        assert!(
            graph.nodes.iter().any(
                |node|
                    node.id ==
                    "concept:resource_exhaustion"
            )
        );

        assert!(
            graph.nodes.iter().any(
                |node|
                    node.id ==
                    "concept:protocol_parsing"
            )
        );

        assert!(
            graph.nodes.iter().any(
                |node|
                    node.id ==
                    "evidence:mini-redis/src/frame.rs"
            )
        );

        // assert on ClaimOf
        // assert on SupportedBy
        // assert on RelatedTo
    }
}

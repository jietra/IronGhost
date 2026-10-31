// src/core/mission_knowledge.rs

use std::collections::{HashMap, HashSet};
use anyhow::{ensure, Result};
use serde::{Serialize, Deserialize};

use crate::core::mission_assessment::AssessmentArtifact;

//
// -------------------------------------
// Mission Knowledge
// -------------------------------------
//

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MissionKnowledge {
    pub source_tree_list    : String,                           // all raw sources
    pub source_visited      : HashSet<String>,                  // no duplicates with hash set
    
    pub cards               : HashMap<String, KnowledgeCard>,   // all concepts/knowledge in cards
    pub current_assessment  : Option<AssessmentArtifact>,       // "index" of knowledge as an updated, structured assessment
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KnowledgeCard {
    pub id          : String,
    pub title       : String,
    pub status      : CardStatus,
    pub content     : String,
    pub references  : Vec<String>,  // raw sources or artifact ids
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CardStatus {
    New,
    Investigating,
    Supported,
    Refuted,
    Resolved,
}

impl Default for CardStatus {
    fn default() -> Self { Self::New }
}

impl MissionKnowledge {

    pub fn visited_sources_to_prompt( &self ) -> String {

        if self.source_visited.is_empty() {
            return "No source visited yet.".into();
        }

        let mut sources: Vec<&String> =
            self.source_visited
                .iter()
                .collect::<Vec<_>>();

        sources.sort();

        sources
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_card(&self, card_id: &str) -> Option<&KnowledgeCard> {
        self.cards.get(card_id)
    }

    pub fn get_card_cloned(
        &self,
        card_id: &str,
    ) -> Option<KnowledgeCard> {
        self.cards.get(card_id).cloned()
    }

    pub fn contains_card(&self, card_id: &str) -> bool {
        self.cards.contains_key(card_id)
    }

    pub fn upsert_card(
        &mut self,
        card: KnowledgeCard,
    ) -> Result<Option<KnowledgeCard>> {
        ensure!(
            !card.id.trim().is_empty(),
            "cannot insert a KnowledgeCard with an empty id"
        );

        ensure!(
            !card.title.trim().is_empty(),
            "cannot insert KnowledgeCard {:?} with an empty title",
            card.id
        );

        let card_id = card.id.clone();

        Ok(self.cards.insert(card_id, card))
    }

    pub fn remove_card(
        &mut self,
        card_id: &str,
    ) -> Option<KnowledgeCard> {
        self.cards.remove(card_id)
    }

}
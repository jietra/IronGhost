// src/core/mission_knowledge.rs

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::mission_assessment::{AssessmentArtifact, ObserverOutput};

//
// -------------------------------------
// Mission Knowledge
// -------------------------------------
//

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MissionKnowledge {
    pub source_tree_list    : String,                           // all raw sources
    pub artifact_ids        : Vec<String>,                      // generated artifacts
    pub cards               : HashMap<String, KnowledgeCard>,   // all concepts
    
    pub current_assessment  : Option<AssessmentArtifact>        // "index" knowledge in a structured assessment

    //actions?   log actions, at least what was seen
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
    fn default() -> Self { CardStatus::New }
}

/*
impl KnowledgeCard {

    pub fn update_from(
        &mut self,
        observation: &ObserverOutput,
    ) {
        todo!()
    }

}
*/
// src/core/mission_knowledge.rs

// WIP

use serde::{Serialize, Deserialize};

//
// -------------------------------------
// Mission Knowledge
// -------------------------------------
//

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MissionKnowledge {
    pub source_tree_list    : String,
    //pub source_ids          : Vec<Source>,  // updated when user adds a raw source
    pub artifact_ids        : Vec<String>,  // updated with mission's advancement
    pub wiki_ids            : Vec<String>,  // updated on a rolling basis (with mission's advancement)
    //pub current_findings    : Vec<Finding>,
    //actions
    //assessment(s)
}
/*
pub struct MissionKnowledge {
    pub observations        : Vec<Observation>,
    pub active_hypotheses   : Vec<Hypothesis>,
    pub findings            : Vec<Finding>,

    pub open_questions      : Vec<String>,
}
*/

/*
/// Source: raw external source of truth, managed solely by user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub id          : String,
    pub name        : String,
    pub source_type : SourceType,
    pub location    : String,
    pub content     : String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SourceType {
    Repository,
    WebPage,
    Pdf,
    Markdown,
}
*/
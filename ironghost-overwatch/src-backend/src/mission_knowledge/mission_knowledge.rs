// src/mission_knowledge/mission_knowledge.rs

use serde::{Serialize, Deserialize};
use crate::core::board::Mission;

pub struct MissionKnowledge {
    pub mission_id          : String,
    pub mission_prompt      : String,

    pub source_ids          : Vec<String>,  // updated when user adds a raw source
    pub artifact_ids        : Vec<String>,  // updated with mission's advancement
    pub wiki_ids            : Vec<String>,  // updated on a rolling basis (with mission's advancement)
    pub current_findings    : Vec<Finding>,
}

//
// -------------------------------------
// Raw source: external source of truth
// managed solely by user
// -------------------------------------
//

pub enum SourceType {
    Repository,
    WebPage,
    Pdf,
    Markdown,
}

pub struct Source {
    pub id          : String,
    pub name        : String,
    pub source_type : SourceType,
    pub location    : String,
    pub content     : String,
}

//
// -------------------------------------
// Investigation/reasoning state
// -------------------------------------
//

pub struct StrategistOutput {
    pub investigation_state: InvestigationState,
    pub mission_graph      : Option<Mission>,
}

// investigation state = grammatically constrained output of strategist
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvestigationState {
    pub analytics           : Vec<String>,
    pub observations        : Vec<Observation>,
    pub hypotheses          : Vec<Hypothesis>,
    pub findings            : Vec<Finding>,
    pub next_move           : Option<NextMove>,
}

// Observation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Observation {
    pub statement   : String,
    pub evidence_ids: Vec<String>,
}

// Hypothesis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hypothesis {
    pub statement   : String,
    pub confidence  : u8,
    pub evidence_ids: Vec<String>,
}

// Finding
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Finding {
    pub statement   : String,
    pub severity    : Severity,
    pub evidence_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
    NA,
}

// Experiment, investigate further...
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NextMove {
    pub intention: String,
    pub action   : Action,
    pub source_id: Option<String>,
    pub target   : Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    FolderExplorer,
    FileReader,
    WebSearcher,
    WebReader,
    Finish,
}
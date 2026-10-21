// src/core/mission.rs

use serde::{Serialize, Deserialize};

//use crate::stores::source_store::FolderTree;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Mission {
    pub id          : String,
    pub prompt      : String,

    pub graph       : MissionGraph,
    pub knowledge   : MissionKnowledge,

    pub status      : MissionStatus,
}

impl Mission {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MissionStatus {
    Draft,
    AwaitingContext,
    AwaitingGo,
    Running,
    Completed,
    Aborted,
}

impl Default for MissionStatus {
    fn default() -> Self {
        MissionStatus::Draft
    }
}

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

//
// -------------------------------------
// Mission Graph
// -------------------------------------
//

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MissionGraph {
    pub nodes: Vec<MissionNode>,
    pub edges: Vec<MissionEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionEdge {
    pub from    : String,
    pub to      : String,
    pub relation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionNode {
    pub id   : String,
    pub kind : String, // "goal", "subgoal", "task", "asset", "vuln", "risk"
    pub title: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub worker     : Option<String>, // E.g.: "coder", "sentinel", "executor", "tool:nmap"

    //#[serde(default)]
    //pub status     : NodeStatus,  // placeholder
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,

    Running {
        started_at: i64,
    },

    Succeeded {
        attempts: usize,
        score   : Option<u8>,
    },

    Failed {
        attempts: usize,
        error   : String,
    },
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Pending
    }
}
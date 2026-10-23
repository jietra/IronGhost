// src/core/mission_assessment.rs

// WIP

use serde::{Serialize, Deserialize};
use crate::core::{
    mission::{Mission, MissionGraph},
    mission_knowledge::MissionKnowledge
};

pub struct AssessmentContext {
    pub mission_prompt   : String,
    pub source_tree_list : String,

    pub iteration        : u32,

    pub knowledge        : MissionKnowledge,
}

//
// -------------------------------------
// Investigation/reasoning state
// -------------------------------------
//

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategistOutput {   // only for parsing purposes: to facilitate llm response parsing into relevant struct
    pub analysis        : String,

    pub observations    : Vec<Observation>,
    pub hypotheses      : Vec<Hypothesis>,
    
    pub next_action     : NextAction,
    pub mission_graph   : Option<MissionGraph>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NextAction {
    pub intention       : String,
    pub action          : String,
    pub target          : String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ObserverOutput {
    pub source_id   : String,

    pub observations: Vec<Observation>,
    pub hypotheses  : Vec<Hypothesis>,
}

// investigation state = grammatically constrained output of strategist
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InvestigationArtifact {
    pub mission_id      : String,
    pub iteration       : u32,

    pub analysis        : String,
    pub observations    : Vec<Observation>,
    pub hypotheses      : Vec<Hypothesis>,
    pub next_move       : Option<NextMove>,
}

// Observation
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Observation {
    pub statement   : String,
    pub evidence_ids: Vec<String>,
}

impl Observation {
    pub fn to_prompt(&self) -> String {
        let evidence_ids = self.evidence_ids.iter().map(|e| format!("[{}]", e)).collect::<Vec<_>>().join(",");
        
        format!(
            "- [{}] {}",
            evidence_ids,
            self.statement
        )
    }
}

pub fn observations_to_prompt(
    observations: &[Observation]
) -> String {

    observations
        .iter()
        .map(|o| o.to_prompt())
        .collect::<Vec<_>>()
        .join("\n")
}


// Hypothesis
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Hypothesis {
    pub statement   : String,
    pub confidence  : u8,
    pub evidence_ids: Vec<String>,
}

impl Hypothesis {

    pub fn to_prompt(&self) -> String {
        let evidence_ids = self.evidence_ids.iter().map(|e| format!("[{}]", e)).collect::<Vec<_>>().join(",");

        format!(
            "- [{}] (confidence={}%) {}",
            evidence_ids,
            self.confidence,
            self.statement,
        )
    }
}



// Finding
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Finding {
    pub statement   : String,
    pub confidence  : u8,
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
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NextMove {
    pub intention: String,
    pub action   : Action,
    //pub source_id: Option<String>,
    //pub target   : Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    FolderExplorer,
    FileReader {
        path: String,
    },
    WebSearcher {
        query: String,
    },
    WebReader {
        url: String,
    },
    AskForMoreContext {
        question: String,
    },
    Finish,
}

impl Default for Action {
    fn default() -> Self {
        Action::Finish
    }
}

pub struct AssessmentPromptBuilder;

impl AssessmentPromptBuilder {

    pub fn build_initial(
        mission: &Mission,
        max_iterations: u32
    ) -> String {

        format!(
r#"
You are Strategist.

MISSION
========

{}

PHASE
=====

Mission Assessment

ITERATION
=========

0 / {}

AVAILABLE SOURCES
=================

{}

Your task is NOT to execute operations.

Your task is to understand the environment.

Produce:

- assessment
- observations
- hypotheses
- next_move

"#,
            mission.prompt,
            max_iterations,
            mission.knowledge.source_tree_list,
        )
    }

    pub fn build_iteration(
        mission: &Mission,
        previous: &InvestigationArtifact,
        observer_output: &ObserverOutput,
        max_iterations: u32
    ) -> String {
        let previous_observations =
            previous.observations
                .iter()
                .map(Observation::to_prompt)
                .collect::<Vec<_>>()
                .join("\n");

        let previous_hypotheses =
            previous.hypotheses
                .iter()
                .map(Hypothesis::to_prompt)
                .collect::<Vec<_>>()
                .join("\n");

        let observer_output_as_str =
            observer_output.observations
                .iter()
                .map(Observation::to_prompt)
                .collect::<Vec<_>>()
                .join("\n");

        format!(
r#"
You are Strategist.

MISSION
========

{}

PHASE
=====

Mission Assessment

ITERATION
=========

{} / {}

PREVIOUS OBSERVATIONS
=====================

{}

PREVIOUS HYPOTHESES
===================

{}

NEW INFORMATION
===============

{}

TASK
====

- Update your assessment.
- Update observations.
- Update hypotheses.
- Choose next_move.

"#,
            mission.prompt,
            previous.iteration+1,
            max_iterations,
            previous_observations,
            previous_hypotheses,
            observer_output_as_str
        )
    }

}


// src/core/mission_assessment.rs

use serde::{Serialize, Deserialize};
use std::fmt;
use crate::core::mission::MissionGraph;

//
// --------------------------------
// Assessment context
// = memory (in assessment loop)
// --------------------------------
//

pub struct AssessmentContext {
    pub mission_prompt      : String,
    pub iteration           : u32,
    pub budget              : u32,
    pub source_tree_list    : String,
    pub previous_assessment : Option<AssessmentArtifact>,
    pub observer_output     : Option<ObserverOutput>

    //pub relevant_cards
    //pub knowledge        : MissionKnowledge,
}

impl AssessmentContext {

    pub fn to_prompt(&self) -> String {
        match self.iteration {
            0 => {
                format!(
r#"
You are Strategist.

MISSION:
========
{}

PHASE: Mission Assessment
======

ITERATION: 0 / {} 
==========

AVAILABLE SOURCES
=================
{}

TASK:
====
Your task is NOT to execute operations.
Your task is to understand the environment.
Produce:
- an updated assessment
- next_move
"#,
                    self.mission_prompt,
                    self.budget,
                    self.source_tree_list,
                )
            }

            _ => {
                let previous_assessment_str: String =
                    self.previous_assessment
                        .as_ref()
                        //.map(|a| a.analysis.as_str())
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "No previous assessment.".into());
                
                let observer_output_str: String =
                    self.observer_output
                        .as_ref()
                        //.map(|a| serde_json::to_string_pretty(a).unwrap())
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "No new information.".into());

                format!(
r#"
You are Strategist.

MISSION:
========
{}

PHASE: Mission Assessment 
======

ITERATION: {} / {}
==========
You have {} iteration(s) left to complete your assessment.

PREVIOUS ASSESSMENT:
====================
{}

NEW INFORMATION:
================
{}

TASK:
=====
- Update your assessment.
- Choose your next_move.

"#,
                    self.mission_prompt,
                    self.iteration,
                    self.budget-1,
                    self.budget.saturating_sub(self.iteration+1),
                    previous_assessment_str,
                    observer_output_str
                )
            }
        }
    }

}

//
// -------------------------------------
// Assessment artifact
// = assessment note at time/iteration "t"
// -------------------------------------
//

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssessmentArtifact {     // parsed from StrategistOutput

    pub iteration       : u32,      // at time "t"
    
    /* ------------------------------------ */
    pub analysis        : String,           //
    pub next_move       : NextAction,       //
    /* ------------------------------------ */

}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StrategistOutput {       // facilitate llm response parsing into relevant struct
    
    /* ------------------------------------ */
    pub analysis        : String,           //
    pub next_move       : NextAction,       //
    /* ------------------------------------ */

    pub mission_graph   : Option<MissionGraph>,

}

//
// -------------------------------------
// Action
// = tool calling
// -------------------------------------
//

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NextAction {
    pub intention       : String,
    pub action          : Action,
    pub target          : String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    FolderExplorer,
    FileReader,
    WebSearcher,
    WebReader,
    AskForMoreContext,
    Finish,
}

impl Default for Action {
    fn default() -> Self {
        Action::AskForMoreContext
    }
}

//
// -------------------------------------
// Observation worker output
// -------------------------------------
//

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ObserverOutput {
    pub source_id   : String,

    pub observations: Vec<Observation>,
    pub hypotheses  : Vec<Hypothesis>,
}

// Observation
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Observation {
    pub statement   : String,
    pub evidence_ids: Vec<String>,
}

// Hypothesis
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Hypothesis {
    pub statement   : String,
    pub confidence  : u8,
    pub evidence_ids: Vec<String>,
}

// -------------------------------------
// Prompt rendering/projection
// -------------------------------------

impl fmt::Display for Observation {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        if self.evidence_ids.is_empty() {

            write!(f, "- {}", self.statement)

        } else {

            write!(
                f,
                "- [{}] {}",
                self.evidence_ids.join(", "),
                self.statement
            )

        }

    }
}

impl fmt::Display for Hypothesis {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(
            f,
            "- (confidence={}%) {}",
            self.confidence,
            self.statement,
        )

    }
}

impl fmt::Display for ObserverOutput {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        writeln!(f, "   SOURCE: {}", self.source_id)?;
        writeln!(f)?;

        writeln!(f, "   OBSERVATIONS:")?;
        for observation in &self.observations {
            writeln!(f, "       {observation}")?;
        }

        writeln!(f)?;
        writeln!(f, "   HYPOTHESES")?;
        for hypothesis in &self.hypotheses {
            writeln!(f, "       {hypothesis}")?;
        }

        Ok(())
    }
}

impl fmt::Display for AssessmentArtifact {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(f, "{}", self.analysis)

    }

}

/*
impl Observation {
    pub fn to_prompt(&self) -> String {
        let evidence_ids: String = self.evidence_ids.iter().map(|e| format!("[{}]", e)).collect::<Vec<_>>().join(",");
        
        format!(
            "- [{}] {}",
            evidence_ids,
            self.statement
        )
    }
}

impl Hypothesis {
    pub fn to_prompt(&self) -> String {
        let evidence_ids: String = self.evidence_ids.iter().map(|e| format!("[{}]", e)).collect::<Vec<_>>().join(",");

        format!(
            "- [{}] (confidence={}%) {}",
            evidence_ids,
            self.confidence,
            self.statement,
        )
    }
}
*/

/*
pub fn observations_to_prompt(
    observations: &[Observation]
) -> String {

    observations
        .iter()
        .map(|o| o.to_prompt())
        .collect::<Vec<_>>()
        .join("\n")
}

//
// -------------------------------------
// Investigation/reasoning state
// -------------------------------------
//

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
*/

/*
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
*/

/*
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
*/
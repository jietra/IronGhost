// src/core/mission_assessment.rs

use serde::{Serialize, Deserialize};
use std::fmt;
use crate::core::mission::{Mission, MissionGraph};

//
// --------------------------------
// Assessment context
// = memory (in assessment loop)
// --------------------------------
//

pub struct AssessmentContext {
    pub mission             : Mission,
    pub observer_output     : Option<ObserverOutput>,

    pub iteration           : u32,
    pub budget              : u32,
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
                    self.mission.prompt,
                    self.budget-1,
                    self.mission.knowledge.source_tree_list,
                )
            }

            _ => {
                let previous_assessment_str: String =
                    self.mission.knowledge.current_assessment
                        .as_ref()
                        //.map(|a| a.analysis.as_str())
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "No previous assessment.".into());
                
                let visited_sources: String =
                    self.mission
                        .knowledge
                        .visited_sources_to_prompt();
                
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
After this iteration, you will have {} iteration(s) left to complete your assessment.

AVAILABLE SOURCES
=================
{}

VISITED SOURCES
=================
{}

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
                    self.mission.prompt,
                    self.iteration,
                    self.budget-1,
                    self.budget.saturating_sub(self.iteration+1),
                    self.mission.knowledge.source_tree_list,
                    visited_sources,
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

    pub iteration           : u32,      // at time "t"
    
    /* ---------------------------------------- */
    pub assessment_report   : String,           //
    pub next_move           : NextAction,       //
    /* ---------------------------------------- */

}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StrategistOutput {       // facilitate llm response parsing into relevant struct
    
    /* ---------------------------------------- */
    pub assessment_report   : String,           //
    pub next_move           : NextAction,       //
    /* ---------------------------------------- */

    pub mission_graph       : Option<MissionGraph>,

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

        write!(f, "{}", self.assessment_report)

    }

}
// src/assessment/context.rs

use crate::{core::mission::Mission, observation::observation::ObserverOutput};

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
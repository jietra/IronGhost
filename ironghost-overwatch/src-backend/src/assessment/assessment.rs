// src/assessment/assessment.rs

use serde::{Serialize, Deserialize};
use std::fmt;

use crate::{
    assessment::action::NextAction,
    core::mission::MissionGraph
};

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

    pub mission_graph       : Option<MissionGraph>,

}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StrategistOutput {       // facilitate llm response parsing into relevant struct
    
    /* ---------------------------------------- */
    pub assessment_report   : String,           //
    pub next_move           : NextAction,       //
    /* ---------------------------------------- */

    pub mission_graph       : Option<MissionGraph>,

}

impl fmt::Display for AssessmentArtifact {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(f, "{}", self.assessment_report)

    }

}
// src/observation/observation.rs

use serde::{Serialize, Deserialize};
use std::fmt;

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

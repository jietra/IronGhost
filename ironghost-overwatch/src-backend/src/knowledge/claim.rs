// src/knowledge/claim.rs

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KnowledgeClaim {
    pub statement   : String,
    pub status      : ClaimStatus,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClaimStatus {
    Hypothesis,
    Supported,
    Confirmed,
    Weakened,
    Refuted,
    Undetermined,
}

impl Default for ClaimStatus {
    fn default() -> Self { Self::Undetermined }
}
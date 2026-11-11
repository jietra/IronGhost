// src/knowledge/card.rs

use std::collections::HashSet;

use anyhow::{ensure, Result};
use serde::{Serialize, Deserialize};

use crate::{
    knowledge::{
        claim::KnowledgeClaim,
        concept::ConceptRef,
    },
    observation::observation::ObserverOutput
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KnowledgeCard {
    pub id          : String,
    pub title       : String,
    pub status      : CardStatus,
    pub summary     : String,
    pub claims      : Vec<KnowledgeClaim>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KnowledgeCardDraft {     // knowledge card proposition from LLM
    pub id      : String,
    pub title   : String,
    pub status  : CardStatus,
    pub summary : String,
    pub claims  : Vec<KnowledgeClaim>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CardStatus {
    New,
    Investigating,
    Resolved,
}

impl Default for CardStatus {
    fn default() -> Self { Self::New }
}

impl KnowledgeCard {
    pub fn try_from_worker_output(
        draft           : KnowledgeCardDraft,
        concept         : &ConceptRef,
        current_card    : Option<&KnowledgeCard>,
        observer_output : &ObserverOutput,
    ) -> Result<Self> {
        
        ensure!(
            draft.id == concept.id,
            "knowledge worker returned card id {:?}, expected {:?}",
            draft.id,
            concept.id,
        );

        ensure!(
            draft.title == concept.title,
            "knowledge worker returned title {:?}, expected {:?}",
            draft.title,
            concept.title,
        );

        ensure!(
            !draft.summary.trim().is_empty(),
            "KnowledgeCard {:?} has an empty summary",
            draft.id,
        );

        validate_claims(&draft)?;

        validate_evidence_ids(
            &draft,
            current_card,
            observer_output,
        )?;

        Ok(draft.into())
    }
}

impl From<KnowledgeCardDraft> for KnowledgeCard {
    fn from(draft: KnowledgeCardDraft) -> Self {
        Self {
            id      : draft.id,
            title   : draft.title,
            status  : draft.status,
            summary : draft.summary,
            claims  : draft.claims,
        }
    }
}

fn validate_evidence_ids(
    draft           : &KnowledgeCardDraft,
    current_card    : Option<&KnowledgeCard>,
    observer_output : &ObserverOutput,
) -> Result<()> {
    let mut allowed: HashSet<&str> = HashSet::new();

    // main source of new artifact
    allowed.insert(observer_output.source_id.as_str());

    // explicit evidence provided by observations
    for observation in &observer_output.observations {
        for evidence_id in &observation.evidence_ids {
            allowed.insert(evidence_id.as_str());
        }
    }

    // explicit evidence provided by hypotheses
    for hypothesis in &observer_output.hypotheses {
        for evidence_id in &hypothesis.evidence_ids {
            allowed.insert(evidence_id.as_str());
        }
    }

    // explicit evidence already in previous card
    if let Some(current_card) = current_card {
        for claim in &current_card.claims {
            for evidence_id in &claim.evidence_ids {
                allowed.insert(evidence_id.as_str());
            }
        }
    }

    for claim in &draft.claims {
        for evidence_id in &claim.evidence_ids {
            ensure!(
                allowed.contains(evidence_id.as_str()),
                "KnowledgeWorker invented evidence id {:?} \
                 in card {:?}",
                evidence_id,
                draft.id,
            );
        }
    }

    Ok(())
}

fn validate_unique_claims(
    draft: &KnowledgeCardDraft,
) -> Result<()> {
    let mut seen: HashSet<String> = HashSet::new();

    for claim in &draft.claims {
        let normalized: String = claim
            .statement
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        ensure!(
            seen.insert(normalized),
            "KnowledgeCard {:?} contains duplicate claims",
            draft.id,
        );
    }

    Ok(())
}

fn validate_claims(
    draft: &KnowledgeCardDraft,
) -> Result<()> {
    for (index, claim) in draft.claims.iter().enumerate() {
        ensure!(
            !claim.statement.trim().is_empty(),
            "KnowledgeCard {:?} claim {} has an empty statement",
            draft.id,
            index,
        );
    }

    validate_unique_claims(draft)?;

    Ok(())
}
// src/knowledge_manager/card_prompt.rs

use anyhow::{Context, Result};

use crate::observation::observation::ObserverOutput;
use crate::knowledge::card::KnowledgeCard;

use super::card_extractor::ConceptRef;

pub fn build_card_upsert_prompt(
    mission_prompt  : &str,
    concept         : &ConceptRef,
    current_card    : Option<&KnowledgeCard>,
    observer_output : &ObserverOutput,
) -> Result<String> {
    let current_card_json: String = match current_card {
        Some(card) => serde_json::to_string_pretty(card)
            .context("failed to serialize current KnowledgeCard")?,

        None => "null".to_string(),
    };

    let observer_output_json: String =
        serde_json::to_string_pretty(observer_output)
            .context("failed to serialize ObserverOutput")?;

    Ok(format!(
        r#"
MISSION
=======
{mission_prompt}

KNOWLEDGE CONCEPT
=================
ID: {concept_id}
TITLE: {concept_title}

CURRENT CARD
============
{current_card_json}

NEW OBSERVER ARTIFACT
=====================
{observer_output_json}

TASK
====
Produce the complete current state of this Knowledge Card
after considering the new Observer Artifact.
"#,
        mission_prompt = mission_prompt,
        concept_id = concept.id,
        concept_title = concept.title,
        current_card_json = current_card_json,
        observer_output_json = observer_output_json,
    ))
}

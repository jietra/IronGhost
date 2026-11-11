// src/knowledge_manager/card_extractor.rs

use crate::observation::observation::ObserverOutput;
use crate::knowledge::concept::{
    ConceptRef,
    extract_concepts,
    deduplicate_concepts,
};

pub fn extract_observer_concepts(observer_output: &ObserverOutput) -> Vec<ConceptRef> {
    let mut concepts: Vec<ConceptRef> = Vec::new();

    for observation in &observer_output.observations {
        concepts.extend(
            extract_concepts(&observation.statement)
        );
    }

    for hypothesis in &observer_output.hypotheses {
        concepts.extend(
            extract_concepts(&hypothesis.statement)
        );
    }

    deduplicate_concepts(concepts)
}
// src/knowledge_manager/concept.rs

use std::collections::{HashSet, HashMap};
use std::sync::LazyLock;
use regex::Regex;

static WIKI_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[([^\[\]]+)\]\]")
        .expect("wiki link regex must be valid")
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConceptRef {
    pub id   : String,
    pub title: String,
}

pub fn concept_to_id(title: &str) -> String {
    let mut id                    : String = String::new();
    let mut previous_was_separator: bool   = false;

    for character in title.trim().to_lowercase().chars() {
        if character.is_alphanumeric() {
            id.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !id.is_empty() {
            id.push('_');
            previous_was_separator = true;
        }
    }

    while id.ends_with('_') {
        id.pop();
    }

    id
}

pub fn extract_concepts(text: &str) -> Vec<ConceptRef> {
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut concepts: Vec<ConceptRef> = Vec::new();

    for captures in WIKI_LINK_REGEX.captures_iter(text) {
        let Some(captured_title) = captures.get(1) else {
            continue;
        };

        let title: &str = captured_title.as_str().trim();

        if title.is_empty() {
            continue;
        }

        let id: String = concept_to_id(title);

        if id.is_empty() || !seen_ids.insert(id.clone()) {
            continue;
        }

        concepts.push(ConceptRef {
            id,
            title: title.to_string(),
        });
    }

    concepts.sort_by(|left, right| left.id.cmp(&right.id));

    concepts
}

pub fn deduplicate_concepts(
    concepts: impl IntoIterator<Item = ConceptRef>,
) -> Vec<ConceptRef> {
    let mut unique: HashMap<String, ConceptRef> =
        HashMap::new();

    for concept in concepts {
        let key: String = normalize_concept_title(
            &concept.title,
        );

        unique.entry(key).or_insert(concept);
    }

    let mut concepts: Vec<ConceptRef> =
        unique.into_values().collect();

    concepts.sort_by(|a, b| {
        a.title.cmp(&b.title)
    });

    concepts
}

fn normalize_concept_title(title: &str,) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_deduplicates_concepts() {
        let text: &str = r#"
            The main concern is [[Memory Exhaustion]].

            It is related to [[Frame Parsing]] and
            [[Memory Exhaustion]].
        "#;

        let concepts: Vec<ConceptRef> = extract_concepts(text);

        assert_eq!(
            concepts,
            vec![
                ConceptRef {
                    id: "frame_parsing".into(),
                    title: "Frame Parsing".into(),
                },
                ConceptRef {
                    id: "memory_exhaustion".into(),
                    title: "Memory Exhaustion".into(),
                },
            ]
        );
    }

    #[test]
    fn creates_normalized_ids() {
        assert_eq!(
            concept_to_id("  Pub/Sub Channel Lifecycle  "),
            "pub_sub_channel_lifecycle"
        );

        assert_eq!(
            concept_to_id("Frame   Parsing"),
            "frame_parsing"
        );
    }

    #[test]
    fn ignores_empty_links() {
        let concepts: Vec<ConceptRef> = extract_concepts("[[]] [[   ]]");

        assert!(concepts.is_empty());
    }
}
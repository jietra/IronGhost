// src/bin/test_web_reader.rs

use backend::knowledge_manager::card_extractor::extract_concepts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let text: &str = r#"
Investigation focuses on
[[Memory Exhaustion]]
and
[[Frame Parsing]].
"#;

    println!("{:?}",
        extract_concepts(text)
    );

    Ok(())
}
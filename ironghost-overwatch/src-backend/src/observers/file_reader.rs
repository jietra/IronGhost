// src/observers/file_reader.rs

use serde::{Serialize, Deserialize};
use std::path::Path;

//
// =========================
// IronGhost Artifacts
// =========================
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileArtifact {
    pub path      : String,
    pub content   : String,
    pub size_bytes: usize,
}

//
// =========================
// Tool
// =========================
//

pub struct FileReader;

impl FileReader {

    pub async fn execute( path: &Path ) -> anyhow::Result<FileArtifact> {

        let content: String = tokio::fs::read_to_string( path ).await?;

        Ok(
            FileArtifact {
                path      : path.to_string_lossy().to_string(),
                size_bytes: content.len(),
                content,
            }
        )
    }
}
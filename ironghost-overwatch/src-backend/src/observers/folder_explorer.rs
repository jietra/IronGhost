// src/observers/folder_explorer.rs

use serde::{Serialize, Deserialize};
use std::path::Path;
use walkdir::WalkDir;

//
// =========================
// IronGhost Artifacts
// =========================
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderArtifact {
    pub root_path: String,
    pub entries  : Vec<FolderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub relative_path: String,
    pub kind         : EntryKind,
    pub size_bytes   : u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryKind {
    File,
    Directory,
}

//
// =========================
// Tool
// =========================
//

pub struct FolderExplorer;

impl FolderExplorer {

    pub fn execute( root: &Path ) -> anyhow::Result<FolderArtifact> {

        let mut entries: Vec<FolderEntry> = Vec::new();

        for entry in WalkDir::new( root ) {
            let entry        : walkdir::DirEntry = entry?;
            let metadata     : std::fs::Metadata = entry.metadata()?;
            let relative_path: String            = entry.path().strip_prefix( root )?
                .to_string_lossy()
                .to_string();

            entries.push(
                FolderEntry {
                    relative_path,
                    kind         : if metadata.is_dir() { EntryKind::Directory } else { EntryKind::File },
                    size_bytes   : metadata.len(),
                }
            );
        }

        Ok(
            FolderArtifact {
                root_path: root.to_string_lossy().to_string(),
                entries,
            }
        )
    }
}
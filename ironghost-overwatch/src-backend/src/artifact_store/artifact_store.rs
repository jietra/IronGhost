// src/artifact_store/artifact_store.rs

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;

#[async_trait]
pub trait ArtifactStore:
    Send + Sync + std::fmt::Debug
{
    async fn save<T>(&self, id: &str, artifact: &T) -> anyhow::Result<()>
    where T: Serialize + Send + Sync;

    async fn load<T>(&self, id: &str              ) -> anyhow::Result<T>
    where T: DeserializeOwned + Send;
}

//
// =========================
// Filesystem storage of artifacts
// =========================
//

#[derive(Debug, Clone)]
pub struct FilesystemArtifactStore {
    root_path: PathBuf,
}

impl FilesystemArtifactStore {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self { root_path: root_path.into() }
    }
}

#[async_trait]
impl ArtifactStore for FilesystemArtifactStore {
    async fn save<T>(&self, artifact_name: &str, artifact: &T) -> anyhow::Result<()>
    where T: Serialize + Send + Sync,
    {
        // path to artifact (saved in .json)
        let path: PathBuf = self.root_path.join( format!("{artifact_name}.json") );

        if let Some( parent ) = path.parent() {
            tokio::fs::create_dir_all( parent ).await?;
        }

        // artifact into json
        let json: String = serde_json::to_string_pretty( artifact )?;

        // write file
        tokio::fs::write( path, json ).await?;

        Ok(())
    }

    async fn load<T>(&self, artifact_name: &str) -> anyhow::Result<T>
    where T: DeserializeOwned + Send,
    {
        let path: PathBuf = self.root_path.join( format!("{artifact_name}.json") );
        let json: String  = tokio::fs::read_to_string( path ).await?;

        Ok( serde_json::from_str(&json)? )
    }
}

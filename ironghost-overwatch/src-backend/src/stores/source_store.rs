// src/stores/source_store.rs

use std::fs;
use std::path::{Path, PathBuf};
//use chrono::Utc;
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStore {
    root_path    : PathBuf,
    max_file_size: u64,

    //tree         : FolderTree,
    //last_update  : u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTree {
    pub entries: Vec<FolderEntry>,
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

impl FolderTree {
    pub fn from_root( root: &Path ) -> Result<FolderTree> {

        let mut entries: Vec<FolderEntry> = Vec::new();

        for entry in WalkDir::new( root ) {
            let entry        : walkdir::DirEntry = entry?;
            let metadata     : fs::Metadata      = entry.metadata()?;
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
            FolderTree {
                entries,
            }
        )
    }

    pub fn get_entry( &self, relative_path: &str ) -> Option<&FolderEntry> {

        self.entries
            .iter()
            .find(|e| e.relative_path == relative_path)
    }

    pub fn contains( &self, relative_path: &str ) -> bool {

        self.get_entry(relative_path)
            .is_some()
    }

    pub fn to_path_listing(&self) -> String {

        let mut entries = self.entries.clone();

        entries.sort_by(|a, b| {
            a.relative_path.cmp(&b.relative_path)
        });

        entries
            .into_iter()
            .filter(|e| !e.relative_path.is_empty())
            .map(|e| {

                match e.kind {

                    EntryKind::Directory =>
                        e.relative_path,

                    EntryKind::File => {

                        let kb =
                            (e.size_bytes as f64 / 1024.0)
                                .ceil() as u64;

                        format!(
                            "{} ({} KB)",
                            e.relative_path,
                            kb
                        )
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

}

impl SourceStore {
    pub fn new(root_path: impl Into<PathBuf>, max_file_size: u64) -> Result<Self, SourceStoreError> {
        let root_path: PathBuf =
            root_path
                .into()
                .canonicalize()
                .map_err(|_| SourceStoreError::CanonicalizeRoot)?;
        
        //let tree: FolderTree = FolderTree::from_root(&root_path).map_err(|_| SourceStoreError::SourceTreeError)?;

        Ok(Self {
            root_path,
            max_file_size,
            //tree,
            //last_update : Utc::now().timestamp_millis() as u64,
        })
    }
/*
    pub fn update(&mut self) -> Result<(), SourceStoreError> {
        let tree: FolderTree =
            FolderTree::from_root(&self.root_path)
                .map_err(|_| SourceStoreError::SourceTreeError)?;
        
        self.tree        = tree;
        self.last_update = Utc::now().timestamp_millis() as u64;

        Ok(())
    }

    pub fn tree(&self) -> &FolderTree {
        &self.tree
    }
*/
    pub fn root(&self) -> &Path {
        self.root_path.as_path()
    }

    pub fn load_file( &self, relative_path: &str ) -> Result<String, SourceStoreError> {

        let canonical_file: PathBuf =
            self.resolve_file(relative_path)?;

        let content: String =
            fs::read_to_string(&canonical_file)
                .map_err(|_| SourceStoreError::ReadFailure)?;

        Ok(content)
    }

    pub fn validate( &self, relative_path: &str ) -> Result<(), SourceStoreError> {
        self.resolve_file(relative_path)?;
        Ok(())
    }

    fn resolve_file( &self, relative_path: &str ) -> Result<PathBuf, SourceStoreError> {
        if relative_path.trim().is_empty() {
            return Err(SourceStoreError::EmptyPath);
        }

        if Path::new(relative_path).is_absolute() {
            return Err(SourceStoreError::PathEscape);
        }

        //if !self.tree.contains(relative_path) { return Err(SourceStoreError::FileNotFound); }

        let requested_path: PathBuf = self.root_path.join(relative_path);

        let canonical_file: PathBuf =
            match requested_path.canonicalize() {

                Ok(path) => path,

                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound =>
                {
                    return Err(SourceStoreError::FileNotFound);
                }

                Err(e)
                    if e.kind() == std::io::ErrorKind::PermissionDenied =>
                {
                    return Err(SourceStoreError::PermissionDenied);
                }

                Err(_) => {
                    return Err(SourceStoreError::CanonicalizeFile);
                }
            };

        if !canonical_file.starts_with(&self.root_path) {
            return Err(SourceStoreError::PathEscape);
        }

        let metadata: fs::Metadata =
            fs::metadata(&canonical_file)
                .map_err(|_| SourceStoreError::MetadataFailure)?;

        if !metadata.is_file() {
            return Err(SourceStoreError::NotAFile);
        }

        if metadata.len() > self.max_file_size {
            return Err(SourceStoreError::FileTooLarge);
        }

        Ok(canonical_file)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceStoreError {

    #[error("empty path")]
    EmptyPath,

    #[error("path escape detected")]
    PathEscape,

    #[error("file not found")]
    FileNotFound,

    #[error("permission denied")]
    PermissionDenied,

    #[error("requested path is not a file")]
    NotAFile,

    #[error("file too large")]
    FileTooLarge,

    #[error("failed to canonicalize source root")]
    CanonicalizeRoot,

    #[error("failed to canonicalize requested file")]
    CanonicalizeFile,

    #[error("failed to read metadata")]
    MetadataFailure,

    #[error("failed to read file")]
    ReadFailure,

    #[error("failed to fetch source tree")]
    SourceTreeError,
}
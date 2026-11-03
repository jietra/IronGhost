// src/observers/folder_explorer.rs

use std::sync::Arc;
use anyhow::Result;
use crate::stores::source_store::{FolderTree, SourceStore};

pub struct FolderExplorer {
    source_store: Arc<SourceStore>,
}

impl FolderExplorer {

    pub fn new( source_store: Arc<SourceStore> ) -> Self {
        Self { source_store }
    }

    pub fn execute( &self ) -> Result<FolderTree> {
        //self.source_store.tree().clone()

        FolderTree::from_root(
            self.source_store.root()
        )
    }

}
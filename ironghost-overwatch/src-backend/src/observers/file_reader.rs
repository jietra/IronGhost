// src/observers/file_reader.rs

use std::sync::Arc;
use crate::stores::source_store::{SourceStore, SourceStoreError};

pub struct FileReader {
    source_store: Arc<SourceStore>,
}

impl FileReader {

    pub fn new( source_store: Arc<SourceStore> ) -> Self {
        Self { source_store }
    }

    pub fn execute( &self, relative_path: &str ) -> Result<String, SourceStoreError> {

        // TODO: log
        
        self.source_store.load_file(relative_path)
    }
    
}
// src/executors/models.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub environment : EnvironmentKind,
    pub runtime     : RuntimeKind,
    pub source      : String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionVerdict {
    pub success     : bool,
    pub stdout      : String,
    pub stderr      : String,
    pub exit_code   : i32,

    //pub duration_ms: u64,
    //pub working_directory: String,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentKind {
    Local,
    Kali,
    //Ubuntu,
    //Windows,
    //ActiveDirectory,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    Python,
    Bash,
    PowerShell,
    Rust,
}

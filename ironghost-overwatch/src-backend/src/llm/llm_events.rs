use serde::Deserialize;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStopReason {
    Eog,
    MaxTokens,
    Cancelled,
    ContextFull,
    TokenizerError,
    DecodeError,
    TokenToPieceError,
    Unknown,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct EndEvent {
    pub event: String,
    pub reason: GenerationStopReason,
}

#[derive(Debug)]
pub enum LlmEvent {
    Content(String),
    End(GenerationStopReason),
}

#[derive(Debug)]
pub struct GenerationOutput {
    pub output: String,
    pub reason: GenerationStopReason,
}

impl GenerationOutput {
    pub fn is_complete(&self) -> bool {
        self.reason == GenerationStopReason::Eog
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("unable to connect to socket: {0}")]
    Connect(#[source] io::Error),

    #[error("socket write error: {0}")]
    Write(#[source] io::Error),

    #[error("socket read error: {0}")]
    Read(#[source] io::Error),

    #[error("socket unexpectedly closed")]
    UnexpectedEof,

    #[error("frame too large: {size} bytes")]
    FrameTooLarge { size: usize },

    #[error("LLM output too large")]
    OutputTooLarge,

    #[error("invalid UTF-8 data: {0}")]
    InvalidUtf8(#[source] std::string::FromUtf8Error),

    #[error("generation timeout")]
    Timeout,
}
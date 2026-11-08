use serde::Deserialize;
use std::io;

use crate::llm::protocol::ProtocolError;

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
    Write(#[source] io::Error),             // can be deleted (I/O in ProtocolError)

    #[error("socket read error: {0}")]
    Read(#[source] io::Error),              // can be deleted (I/O in ProtocolError)

    #[error("socket unexpectedly closed")]
    UnexpectedEof,                          // can be deleted (I/O in ProtocolError)

    #[error("frame too large: {size} bytes")]
    FrameTooLarge { size: usize },          // can be deleted (I/O in ProtocolError)

    #[error("LLM output too large")]
    OutputTooLarge,

    #[error("invalid UTF-8 data: {0}")]
    InvalidUtf8(#[source] std::string::FromUtf8Error),

    #[error("invalid LLM control frame: {0}")]
    InvalidControlFrame(String),

    #[error("remote LLM error: {0}")]
    Remote(String),

    #[error("generation timeout")]
    Timeout,

    #[error("LLM protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}
// src/llm/stream_forwarder.rs

use std::sync::Arc;
use tokio::pin;
use tokio::sync::{Mutex, MutexGuard, broadcast::Sender};
use futures_util::{Stream, StreamExt};
use chrono::Utc;
use anyhow::{Result, anyhow};

use crate::core::board::{Board, BoardEvent};
use crate::llm::{
    llm::LLM,
    llm_events::{LlmError, LlmEvent, GenerationOutput, GenerationStopReason}
};

pub struct StreamForwarder;

impl StreamForwarder {
    pub async fn forward<S>(
        stream  : S,
        tx_board: Sender<BoardEvent>,
        msg_id  : String,
        agent   : String,
    ) -> Result<GenerationOutput, LlmError>
    where
        S: Stream<Item = Result<LlmEvent, LlmError>>,
    {
        // start of stream -> Notif UI via Board
        let _ = tx_board.send(BoardEvent::StreamStart {
            msg_id: msg_id.clone(),
            agent,
        });

        // process stream
        let mut full_output: String = String::new();
        let mut stop_reason: Option<GenerationStopReason> = None;

        // pin stream on stack to authorize .next()
        pin!(stream);

        while let Some(event) = stream.next().await {
            match event {
                Ok(LlmEvent::Content(chunk)) => {
                    full_output.push_str(&chunk);

                    // Forward model content only.
                    let _ = tx_board.send(BoardEvent::StreamChunk {
                        msg_id: msg_id.clone(),
                        delta: chunk,
                    });
                }

                Ok(LlmEvent::End(reason)) => {
                    stop_reason = Some(reason);
                    break;
                }

                Err(error) => {
                    let _ = tx_board.send(BoardEvent::StreamEnd {
                        msg_id: msg_id.clone(),
                    });

                    return Err(error);
                }
            }
        }

        let reason: GenerationStopReason = stop_reason.unwrap_or(
            GenerationStopReason::Unknown
        );

        // send end of stream
        let _ = tx_board.send(BoardEvent::StreamEnd {
            msg_id,
        });

        Ok(GenerationOutput {
            output: full_output,
            reason,
        })
    }

    pub async fn generate(service: &str, prompt: &str, client: &LLM, board: Arc<Mutex<Board>>) -> Result<GenerationOutput, LlmError> {
        let stream = client.stream_generate(&prompt);
        let msg_id  : String = format!("{}-{}", service, Utc::now().timestamp_millis());
        let tx_board: Sender<BoardEvent> = {
            let board_guard: MutexGuard<'_, Board> = board.lock().await;
            board_guard.tx.clone()
        };
        Self::forward(
            stream,
            tx_board,
            msg_id,
            service.into(),
        )
        .await
    }

    pub fn validate_generation(
        role        : &str,
        generation  : GenerationOutput,
    ) -> Result<String> {
        if generation.reason != GenerationStopReason::Eog {
            return Err(anyhow!(
                "[LLM SERVICE] {} generation stopped with reason {:?} \
                after producing {} bytes",
                role,
                generation.reason,
                generation.output.len(),
            ));
        }

        if generation.output.trim().is_empty() {
            return Err(anyhow!(
                "[LLM SERVICE] {} did not send any content",
                role,
            ));
        }

        Ok(generation.output)
    }
}
// src/llm/stream_forwarder.rs

use std::sync::Arc;
use tokio::pin;
use tokio::sync::{Mutex, MutexGuard, broadcast::Sender};
use futures_util::{Stream, StreamExt};
use chrono::Utc;

use crate::core::board::{Board, BoardEvent};
use crate::llm::llm::LLM;

pub struct StreamForwarder;

impl StreamForwarder {
    pub async fn forward<S>(
        stream  : S,
        tx_board: Sender<BoardEvent>,
        msg_id  : String,
        agent   : String,
    ) -> String
    where
        S: Stream<Item = String>,
    {
        // start of stream -> Notif UI via Board
        let _ = tx_board.send(BoardEvent::StreamStart {
            msg_id: msg_id.clone(),
            agent,
        });

        // process stream
        let mut full_output: String = String::new();

        // pin stream on stack to authorize .next()
        pin!(stream);

        while let Some(chunk) = stream.next().await {
            full_output.push_str(&chunk);

            // send each chunk on broadcast channel for UI
            let _ = tx_board.send(BoardEvent::StreamChunk {
                msg_id: msg_id.clone(),
                delta : chunk,
            });
        }

        // send end of stream
        let _ = tx_board.send(BoardEvent::StreamEnd {
            msg_id,
        });

        full_output
    }

    pub async fn generate(service: &str, prompt: &str, client: &LLM, board: Arc<Mutex<Board>>) -> String {
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
}
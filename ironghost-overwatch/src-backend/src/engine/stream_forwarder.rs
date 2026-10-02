// src/engine/stream_forwarder.rs

use tokio::pin;
use futures_util::Stream;
use futures_util::StreamExt;

use tokio::sync::broadcast::Sender;

use crate::core::board::BoardEvent;

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
}
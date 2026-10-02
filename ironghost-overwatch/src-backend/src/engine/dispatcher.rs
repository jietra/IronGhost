// src/engine/dispatcher.rs

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::board::{Board, MissionNode, BoardEvent, BoardMessage};
use crate::engine::registry::AgentRegistry;

use crate::agents::agent_llm::AgentLLM;
use chrono::Utc;
use tokio::pin;
use futures_util::StreamExt;        // needed for .next()

pub struct AgentDispatcher {
    registry: Arc<AgentRegistry>,
}

impl AgentDispatcher {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }

    pub async fn execute(
        &self,
        node: &MissionNode,
        board: Arc<Mutex<Board>>,
    ) -> Result<String, String> {

        // 1. fetch assigned specialized agent and instruction
        let socket_path = self.registry
            .resolve(node.worker.as_deref().unwrap_or("coder"))
            .ok_or_else(|| format!("[DISPATCHER] Unknown worker {:?}", node.worker))?;

        let prompt = node.description.clone().unwrap_or_else(|| node.title.clone());

        println!("[DISPATCHER] Dispatching node [{}] to {}", node.id, socket_path);

        // 2. call specialized agent to handle task
        let client = AgentLLM::new(socket_path);

        // prepare streaming to Board
        let msg_id   = format!("{}-{}", "Runner", Utc::now().timestamp_millis());
        let tx_board = board.lock().await.tx.clone();

        // start of stream -> Notif UI via Board
        let _ = tx_board.send(BoardEvent::StreamStart {
            msg_id: msg_id.clone(),
            agent : "Runner".into(),
        });

        // process stream
        let mut full_output = String::new();
        let stream          = client.stream_generate(&prompt);
           
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
            msg_id: msg_id.clone(),
        });

        if full_output.is_empty() {
            Err(format!("[DISPATCHER] SpecializedAgent on {} did not send any content.", socket_path))
        } else {
            println!("[DISPATCHER] Final code generated for node [{}]", node.id);

            // Notify final output to Board
            let mut b = board.lock().await;
            b.publish(BoardMessage {
                agent    : "Runner".into(),
                content  : format!("Task [{}] completed with result:\n{}\n", node.id, full_output),
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;

            Ok(full_output)
        }
    }
}
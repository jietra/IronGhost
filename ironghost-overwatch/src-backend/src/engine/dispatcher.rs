// src/engine/dispatcher.rs

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::board::{Board, MissionNode, BoardEvent, BoardMessage};
use crate::engine::registry::AgentRegistry;

use crate::agents::agent_llm::AgentLLM;
use chrono::Utc;

use crate::engine::stream_forwarder::StreamForwarder;

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

        // fetch assigned specialized agent and instruction
        let socket_path: &str = self.registry
            .resolve(node.worker.as_deref().unwrap_or("coder"))
            .ok_or_else(|| format!("[DISPATCHER] Unknown worker {:?}", node.worker))?;

        let prompt: String = node.description.clone().unwrap_or_else(|| node.title.clone());

        println!("[DISPATCHER] Dispatching node [{}] to {}", node.id, socket_path);

        // call specialized agent to handle task
        let client: AgentLLM = AgentLLM::new(socket_path);
        let stream = client.stream_generate(&prompt);

        // prepare streaming to Board
        let msg_id  : String = format!("{}-{}", "Runner", Utc::now().timestamp_millis());
        let tx_board: tokio::sync::broadcast::Sender<BoardEvent> = board.lock().await.tx.clone();

        let full_output: String = StreamForwarder::forward(
            stream,
            tx_board,
            msg_id,
            "Runner".into(),
        )
        .await;

        if full_output.is_empty() {
            Err(format!("[DISPATCHER] SpecializedAgent on {} did not send any content.", socket_path))
        } else {
            println!("[DISPATCHER] Final code generated for node [{}]", node.id);

            // Notify final output to Board
            let mut b: tokio::sync::MutexGuard<'_, Board> = board.lock().await;
            b.publish(BoardMessage {
                agent    : "Runner".into(),
                content  : format!("Task [{}] completed with result:\n{}\n", node.id, full_output),
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;

            Ok(full_output)
        }
    }
}
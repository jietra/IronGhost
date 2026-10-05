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

        // specialized agent / worker
        let socket_path: &str = self.registry
            .resolve(node.worker.as_deref().unwrap_or("coder"))
            .ok_or_else(|| format!("[DISPATCHER] unknown worker {:?}", node.worker))?;
        let client: AgentLLM = AgentLLM::new(socket_path);
        println!("[DISPATCHER] dispatching node [{}] to {}", node.id, socket_path);

        // sentinel
        let sentinel_socket: &str = self.registry
            .resolve("sentinel-code")
            .ok_or_else(|| format!("[DISPATCHER] sentinel not found for worker {:?}", node.worker))?;
        let sentinel_client: AgentLLM = AgentLLM::new(sentinel_socket);

        // task description
        let prompt: String = node.description.clone().unwrap_or_else(|| node.title.clone());

        // task handling
        let stream = client.stream_generate(&prompt);
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
            Err(format!("[DISPATCHER] specializedAgent on {} did not send any content.", socket_path))
        } else {
            println!("[DISPATCHER] code generated for node [{}]\n\tdispatching to sentinel {}", node.id, sentinel_socket);
            // sentinel validation
            let sentinel_prompt: String = format!("[TASK OBJECTIVE]:\n\n{}\n[GENERATED CODE]:\n\n{}", prompt, full_output);
            let sentinel_stream = sentinel_client.stream_generate(&sentinel_prompt);
            let sentinel_msg_id  : String = format!("{}-{}", "Sentinel", Utc::now().timestamp_millis());
            let tx_board: tokio::sync::broadcast::Sender<BoardEvent> = board.lock().await.tx.clone();
            let sentinel_full_output: String = StreamForwarder::forward(
                sentinel_stream,
                tx_board.clone(),
                sentinel_msg_id,
                "Sentinel".into(),
            )
            .await;

            // Notify final output to Board
            let mut b: tokio::sync::MutexGuard<'_, Board> = board.lock().await;
            b.publish(BoardMessage {
                agent    : "Runner".into(),
                content  : format!("Task [{}] completed with result:\n{}\nand sentinel output:\n{}", node.id, full_output, sentinel_full_output),
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;

            Ok(full_output)
        }
    }
}
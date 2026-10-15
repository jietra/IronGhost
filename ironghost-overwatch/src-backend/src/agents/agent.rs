// src/agents/agent.rs

use crate::core::board::{BoardMessage, BoardEvent, Board, Mission};
use crate::agents::agent_buffer::AgentBuffer;
use crate::agents::agent_llm::AgentLLM;
use crate::utils::stream_forwarder::StreamForwarder;

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, MutexGuard};
use chrono::Utc;

pub struct Agent {
    pub name  : String,                             // agent's id
    pub buffer: AgentBuffer,                        // agent prompt to be sent to agent's llm service (last board publications since last agent's response)
    pub rx    : broadcast::Receiver<BoardEvent>,    // agent receiver to receive board broadcasts
    pub llm   : AgentLLM,                           // representation of agent's llm service
    pub board : Arc<Mutex<Board>>,                  // board the agent subscribes and publish to
}

impl Agent {
    pub fn new(
        name : &str,
        board: Arc<Mutex<Board>>,
        rx   : broadcast::Receiver<BoardEvent>,
        llm  : AgentLLM,
    ) -> Self {

        Self {
            name  : name.to_string(),
            buffer: AgentBuffer::new(),
            rx,
            llm,
            board,
        }

    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            println!("[AGENT] agent {} running...", self.name);

            self.run().await;
        });
    }

    pub async fn run(mut self) {
        loop {
            let Some(event) = self.recv_event().await
                else { continue; };

            self.handle_event(event).await;
        }
    }

    async fn recv_event( &mut self ) -> Option<BoardEvent> {

        match self.rx.recv().await {

            Ok(event) =>
                Some(event),

            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                eprintln!("[AGENT] WARNING: agent {} skipped {} messages", self.name, skipped);
                None
            }

            Err(broadcast::error::RecvError::Closed) => {
                eprintln!("[AGENT] WARNING: agent {} broadcast channel closed", self.name);
                None
            }
        }
    }

    async fn handle_event( &mut self, event: BoardEvent ) {

        match event {

            BoardEvent::NewMessage(msg) =>
                self.on_message(msg).await,

            BoardEvent::MissionState(mission) =>
                self.on_mission_state(mission).await,

            BoardEvent::ArtifactCreated {mission_id, artifact_id} =>
                self.on_artifact_created(
                    mission_id,
                    artifact_id,
                ).await,

            BoardEvent::InvestigationStateUpdated {mission_id} =>
                self.on_investigation_updated(
                    mission_id
                ).await,

            BoardEvent::StreamStart { .. }
            | BoardEvent::StreamChunk { .. }
            | BoardEvent::StreamEnd { .. } => {}
        }
    }

    async fn on_message( &mut self, msg: BoardMessage ) {
        if msg.agent == self.name { return; }

        self.buffer.push( msg.clone() );

        if msg.content.contains(&format!("!@{}", self.name)) {

            println!("[AGENT] agent {} aknowledges tag", self.name);
            
            // extract prompt from buffer
            let prompt: String = self.buffer.flush(&self.name);

            println!("[AGENT] prompt to be sent to llm service:\n\"{}\"\n", prompt);

            // call service
            let full_response: String = StreamForwarder::generate(
                &self.name,
                &prompt,
                &self.llm,
                self.board.clone(),
            ).await;
            
            // publish final output
            let mut board: MutexGuard<'_, Board> = self.board.lock().await;    //
            board.publish(BoardMessage {
                agent    : self.name.clone(),
                content  : full_response,
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;
        }
    }

    async fn on_mission_state( &mut self, mission: Mission) {
        return;
    }

    async fn on_artifact_created( &mut self, mission_id: String, artifact_id: String) {
        // TODO
        return;
    }

    async fn on_investigation_updated( &mut self, mission_id: String) {
        // TODO
        return;
    }
}

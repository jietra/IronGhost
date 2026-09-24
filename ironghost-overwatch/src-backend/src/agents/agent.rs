use crate::core::board::{BoardMessage, BoardEvent, Board};
use crate::agents::agent_buffer::AgentBuffer;
use crate::agents::agent_llm::AgentLLM;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
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
        while let Ok(event) = self.rx.recv().await {

            println!("[AGENT] message received by agent {}", self.name);

            // Extract message if `NewMessage` event
            let msg = match event {
                BoardEvent::NewMessage(msg) => msg,
                BoardEvent::MissionState(_) => {
                    // ignored for now
                    continue;
                }
            };

            // ignore own messages
            if msg.agent == self.name { continue; }

            self.buffer.push( msg.clone() );

            if msg.content.contains(&format!("@{}", self.name)) {

                println!("[AGENT] agent {} aknowledges tag", self.name);

                let prompt    = self.buffer.flush(&self.name);

                println!("[AGENT] sending prompt \"{}\" to llm service...", prompt);

                let response  = self.llm.generate(&prompt).await;

                println!("[AGENT] response received:\n{}", response);

                let mut board = self.board.lock().await;
                board.publish(BoardMessage {
                    agent    : self.name.clone(),
                    content  : response,
                    timestamp: Utc::now().timestamp_millis() as u64,
                }).await;
            }
        
        }
    }
}

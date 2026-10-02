use crate::core::board::{BoardMessage, BoardEvent, Board};
use crate::agents::agent_buffer::AgentBuffer;
use crate::agents::agent_llm::AgentLLM;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::pin;
use chrono::Utc;
use futures_util::StreamExt;        // needed for .next()

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
            // explicitly handle lagged and closed messages to avoid panic on .recv()
            let event = match self.rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("[AGENT] WARNING: agent {} skipped {} messages", self.name, skipped);
                    continue;   // continue instead of panicking
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    eprintln!("[AGENT] WARNING: agent {} broadcast channel closed", self.name);
                    break;
                }
            };

        //}
        //while let Ok(event) = self.rx.recv().await {

            // Extract message if `NewMessage` event
            let msg = match event {
                BoardEvent::NewMessage(msg) => {
                    println!("[AGENT] message received by agent {}", self.name);
                    msg
                },
                BoardEvent::MissionState(_) => continue,    // ignored for now
                BoardEvent::StreamStart { .. } | BoardEvent::StreamChunk { .. } | BoardEvent::StreamEnd { .. } => continue, // ignore stream from other agents
            };

            // ignore own messages
            if msg.agent == self.name { continue; }

            self.buffer.push( msg.clone() );

            if msg.content.contains(&format!("!@{}", self.name)) {
                println!("[AGENT] agent {} aknowledges tag", self.name);

                // extract prompt from buffer
                let prompt = self.buffer.flush(&self.name);
                println!("[AGENT] prompt to be sent to llm service:\n\"{}\"\n", prompt);

                // prepare streaming response
                let msg_id = format!("{}-{}", self.name, Utc::now().timestamp_millis());
                
                //let tx = self.board.lock().await.tx.clone();
                // get tx quickly without holding the mutex for too long (to avoid deadlocks)
                let tx = {
                    let board_guard = self.board.lock().await;
                    board_guard.tx.clone()
                };

                // start of stream -> Notif UI via Board
                let _ = tx.send(BoardEvent::StreamStart {
                    msg_id: msg_id.clone(),
                    agent : self.name.clone(),
                });
                
                // process stream
                let mut full_response = String::new();
                let llm_stream = self.llm.stream_generate(&prompt);
                pin!(llm_stream);   // pin stream on stack to authorize .next()

                while let Some(chunk) = llm_stream.next().await {
                    full_response.push_str(&chunk);

                    // send each chunk on broadcast channel for UI
                    let _ = tx.send(BoardEvent::StreamChunk {
                        msg_id: msg_id.clone(),
                        delta : chunk,
                    });
                }

                // end of stream
                let _ = tx.send(BoardEvent::StreamEnd {
                    msg_id: msg_id.clone(),
                });

                //let response  = self.llm.generate(&prompt).await;
                //println!("[AGENT] response received:\n{}", response);

                let mut board = self.board.lock().await;    //
                board.publish(BoardMessage {
                    agent    : self.name.clone(),
                    content  : full_response,
                    timestamp: Utc::now().timestamp_millis() as u64,
                }).await;
            }
        }
    }
}

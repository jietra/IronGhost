// src/agents/system_agent.rs

use crate::core::board::{BoardMessage, BoardEvent, Board};
use crate::core::mission::{Mission, MissionGraph, MissionStatus};
use crate::core::mission_knowledge::MissionKnowledge;

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, MutexGuard};
use chrono::Utc;

pub struct SystemAgent {
    pub rx    : broadcast::Receiver<BoardEvent>,    // agent receiver to receive board broadcasts
    pub board : Arc<Mutex<Board>>,                  // board the agent subscribes and publish to
}

impl SystemAgent {
    pub fn new(
        board: Arc<Mutex<Board>>,
        rx   : broadcast::Receiver<BoardEvent>,
    ) -> Self {

        Self { rx, board }

    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            println!("[SYSTEM AGENT] system agent running...");

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
                eprintln!("[SYSTEM AGENT] WARNING: system agent skipped {} messages", skipped);
                None
            }
            Err(broadcast::error::RecvError::Closed) => {
                eprintln!("[SYSTEM AGENT] WARNING: system agent broadcast channel closed");
                None
            }
        }
    }

    async fn handle_event( &mut self, event: BoardEvent ) {

        match event {

            BoardEvent::NewMessage(msg) =>
                self.on_message(msg).await,

            _ => {}
        }
    }

    async fn on_message( &mut self, msg: BoardMessage ) {
        if msg.agent != "Operator" { return; }

        if let Some(prompt) = msg.content.strip_prefix(
            "!Initiate_Mission"
        ) {
            self.create_mission(prompt).await;
        }
    }

    async fn create_mission(
        &mut self,
        prompt: &str,
    ) {

        let mission: Mission = Mission {
            id        : format!(
                "Mission-{}", Utc::now().timestamp_millis()
            ),
            prompt    : prompt.to_string(),
            graph     : MissionGraph::default(),
            knowledge : MissionKnowledge::default(),
            status    : MissionStatus::Draft,
        };

        {
            let mut board: MutexGuard<'_, Board> = self.board.lock().await;

            board.set_current_mission(
                mission.clone()
            );
        }

        let _ =
            self.board
                .lock()
                .await
                .tx
                .send(
                    BoardEvent::MissionState(
                        mission.graph.clone()
                    )
                );

        let _ =
            self.board
                .lock()
                .await
                .tx
                .send(
                    BoardEvent::DispatchCommand {
                        mission_id : mission.id.clone(),
                        to         : "strategist".into(),
                        payload    : "!Initiate_Mission".into(),
                    }
                );

        let mut board: MutexGuard<'_, Board> = self.board.lock().await; 
        board.publish(BoardMessage {
            agent    : "System".into(),
            content  : format!("New mission created with id: {}", mission.id.clone()),
            timestamp: Utc::now().timestamp_millis() as u64,
        }).await;
    }

}

// src/agents/agent.rs

use crate::core::board::{BoardMessage, BoardEvent, Board};
use crate::core::mission::Mission;
use crate::agents::agent_buffer::AgentBuffer;
use crate::agents::agent_llm::AgentLLM;
use crate::core::mission_assessment::{Action, AssessmentPromptBuilder, InvestigationArtifact, NextMove, ObserverOutput};
use crate::utils::stream_forwarder::StreamForwarder;
use crate::stores::source_store::FolderTree;

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, MutexGuard};
use chrono::Utc;
use anyhow::Result;

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

            BoardEvent::DispatchCommand{ mission_id, to, payload } => {
                if let Err(e) = self.on_command(mission_id, to, payload).await {
                    eprintln!("[AGENT] ERROR, command failed: {}", e);
                }
            }

            BoardEvent::StreamStart { .. }
            | BoardEvent::StreamChunk { .. }
            | BoardEvent::StreamEnd { .. } => {}

            _ => {}
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

    async fn on_command( &mut self, mission_id: String, to: String, payload: String ) -> Result<()> {
        if self.name != to { return Ok(()); }

        println!("[AGENT] Agent {}, receiving command: {}", self.name, payload);

        match payload.as_str() {

            "!Initiate_Mission" => {
                
                println!("[AGENT] Agent {}, initiating mission: {}", self.name, mission_id);
                
                self.refresh_source_tree().await?;
                
                self.launch_mission_assessment(mission_id).await?;

            }

            _ => {}
        }

        Ok(())


        // send prompt + source_store.tree()

    }

    async fn refresh_source_tree( &mut self ) -> Result<()> {

        let source_tree: FolderTree = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;

            board
                .registry
                .observer_registry()
                .folder_explorer
                .execute()?
        };

        let source_tree_list: String = source_tree.to_path_listing();

        println!("[AGENT] updated mission tree list:\n {}", source_tree_list);

        {
            let mut board: MutexGuard<'_, Board> = self.board.lock().await;

            board.update_source_tree_list(source_tree_list);
        }
        // => drop(board_guard);

        Ok(())
    }

    async fn launch_mission_assessment( &mut self, mission_id: String ) -> Result<()> {
        
        //
        let max_iterations: u32 = 10;
        // TODO: put this in configuration parameters

        for iteration in 0..max_iterations {

            println!("[AGENT] mission iteration {}", iteration);

            let artifact: InvestigationArtifact = self.run_assessment_iteration( &mission_id, iteration ).await?;

            match &artifact.next_move {

                Some(NextMove { action: Action::Finish, .. }) => {
                    println!("[AGENT] mission assessment completed");
                    break;
                }

                Some(next_move) => {
                    self.execute_next_move( &mission_id, next_move ).await?;
                }

                None => {
                    println!("[AGENT] broken assessment chain - no next_move given at iteration {}", iteration);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn run_assessment_iteration( &mut self, mission_id: &str, iteration: u32 ) -> Result<InvestigationArtifact> {

        // retrieve context
        let mission: Mission = {
            let board = self.board.lock().await;

            board.current_mission()
        };

        //let mission_prompt  : String = mission.prompt.clone();
        //let source_tree_list: String = mission.knowledge.source_tree_list.clone();

        let mut previous: InvestigationArtifact = InvestigationArtifact::default();
        let mut observer_output: ObserverOutput = ObserverOutput::default();

        // build prompt
        let prompt: String = if iteration == 0 {
            AssessmentPromptBuilder::build_initial(&mission)
        } else {
            // TODO: retrieve previous & observer_output
            AssessmentPromptBuilder::build_iteration(&mission, &previous, &observer_output)
        };

        let response: String =
            StreamForwarder::generate(
                &self.name,
                &prompt,
                &self.llm,
                self.board.clone(),
            ).await;
        
        // TODO: publish

        let artifact: InvestigationArtifact =
            serde_json::from_str::<InvestigationArtifact>(
                &response
            )?;

        Ok(artifact)
    }

    async fn execute_next_move( &mut self, mission_id: &str, next_move: &NextMove ) -> Result<()> {
        // TODO
        
        match &next_move.action {

            Action::FileReader { path } => {
                // TODO
            }

            Action::WebSearcher { query } => {
                // TODO
            }

            Action::WebReader { url } => {
                // TODO
            }

            Action::AskForMoreContext { question } => {
                // TODO
            }

            _ => {}
        }

        Ok(())
    }

    async fn on_artifact_created( &mut self, mission_id: String, artifact_id: String) {
        if self.name != "Strategist" {
            return;
        }

        return;

        /*
        let artifact = FilesystemArtifactStore::load(&artifact_id);
        let mission_knowledge = FilesystemMissionStore::load(&mission_id);

        let prompt =
            StrategistPromptBuilder::build(
                mission_knowledge,
                artifact,
            );

        let response =
            StreamForwarder::generate(
                &self.name,
                &prompt,
                &self.llm,
                self.board.clone(),
            ).await;

        */

        // parse InvestigationState

        // save it

        // publish InvestigationStateUpdated
    }

    async fn on_investigation_updated( &mut self, mission_id: String) {
        // TODO
        return;
    }
}
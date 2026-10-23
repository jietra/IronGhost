// src/agents/agent.rs

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, MutexGuard};
use chrono::Utc;
use anyhow::{Result, anyhow};

use crate::core::board::{BoardMessage, BoardEvent, Board};
use crate::core::mission::Mission;
use crate::core::mission_assessment::{Action, AssessmentPromptBuilder, InvestigationArtifact, NextAction, NextMove, ObserverOutput, StrategistOutput};
use crate::agents::agent_buffer::AgentBuffer;
use crate::llm::{llm::LLM, stream_forwarder::StreamForwarder};
use crate::stores::source_store::FolderTree;
use crate::workers::dispatcher::Dispatcher;
use crate::config::SystemRegistry;

pub struct Agent {
    pub name  : String,                             // agent's id
    pub buffer: AgentBuffer,                        // agent prompt to be sent to agent's llm service (last board publications since last agent's response)
    pub rx    : broadcast::Receiver<BoardEvent>,    // agent receiver to receive board broadcasts
    pub llm   : LLM,                                // representation of agent's llm service
    pub board : Arc<Mutex<Board>>,                  // board the agent subscribes and publish to
}

impl Agent {
    pub fn new(
        name : &str,
        board: Arc<Mutex<Board>>,
        rx   : broadcast::Receiver<BoardEvent>,
        llm  : LLM,
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
            let mut prompt: String = self.buffer.flush();

            prompt.push_str(&format!(
                "[YOUR TURN]\nYou are {}. Respond strictly to tasks addressed to you or @All.\n{}:",
                self.name, self.name
            ));

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
                //
                let budget: u32 = 5;
                // we set a budget = max iterations in assessment loop
                // NB: llm context overflow with current design around 7-8 iterations
                // => "full context" will be replaced by a preliminary report
                // (summarizing current loop assessment)
                // (mission knowledge design)
                // TODO: put budget in configuration parameters.
                // OPTIM: fine-grain budgeting with websearch budget, sub-mission budget (depth), next_move batch budget etc.
                //
                
                println!("[AGENT] Agent {}, initiating mission: {}", self.name, mission_id);
                
                self.refresh_source_tree().await?;
                
                self.launch_mission_assessment(mission_id, budget).await?;

            }

            _ => {}
        }

        Ok(())

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

    // mission assessment loop
    async fn launch_mission_assessment( &mut self, mission_id: String, budget: u32 ) -> Result<()> {

        let mut previous: InvestigationArtifact = InvestigationArtifact::default();
        let mut observer_output: ObserverOutput = ObserverOutput::default();

        for iteration in 0..budget {

            println!("[AGENT] mission iteration {}", iteration);

            let artifact: InvestigationArtifact = self.run_assessment_iteration( previous.clone(), observer_output.clone(), iteration, budget ).await?;

            match &artifact.next_move {

                Some(NextMove { action: Action::Finish, .. }) => {
                    println!("[AGENT] mission assessment completed");
                    break;
                }

                Some(next_move) => {
                    println!("[AGENT] executing next move {:?}", next_move);
                    observer_output = self.execute_next_move( &mission_id, next_move ).await?;
                }

                None => {
                    println!("[AGENT] broken assessment chain - no next_move given at iteration {}", iteration);
                    break;
                }
            }

            previous = artifact;
        }

        Ok(())
    }

    async fn run_assessment_iteration( &mut self, previous: InvestigationArtifact, observer_output: ObserverOutput, iteration: u32, max_iterations: u32 ) -> Result<InvestigationArtifact> {

        // retrieve context
        let mission: Mission = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;

            board.current_mission()
        };

        // build prompt
        //let mut prompt: String = self.buffer.flush();
        
        let prompt_suppl: String = if iteration == 0 {
            AssessmentPromptBuilder::build_initial(&mission, max_iterations)
        } else {
            AssessmentPromptBuilder::build_iteration(&mission, &previous, &observer_output, max_iterations)
        };

        //prompt.push_str(&format!("[YOUR TURN]\n"));
        //prompt.push_str(&prompt_suppl);

        let response: String =
            StreamForwarder::generate(
                &self.name,
                &prompt_suppl,
                &self.llm,
                self.board.clone(),
            ).await;

        // parse response
        println!("DEBUG response: {}", &response);

        let json_str: &str = extract_json(&response)
            .ok_or_else(|| anyhow!("unable to extract json"))?;

        let strategist_output: StrategistOutput =
            serde_json::from_str::<StrategistOutput>(
                json_str
            )?;
        
        println!("DEBUG strategist_output done");
        
        let intention: String   = strategist_output.next_action.intention;
        let target   : String   = strategist_output.next_action.target;
        let next_move: NextMove = match strategist_output.next_action.action.as_str() {
            "FileReader" => NextMove {
                intention,
                action: Action::FileReader { path: target }
            },

            // ...

            _ => NextMove::default(),
        };

        println!("next_move: {:?}", next_move);

        let artifact: InvestigationArtifact = InvestigationArtifact {
            mission_id: previous.mission_id,
            iteration,
            analysis: strategist_output.analysis,
            observations: strategist_output.observations,
            hypotheses: strategist_output.hypotheses,
            next_move: Some(next_move),
        };

        println!("artifact: {:?}", artifact);

        // publish
        let mut board: MutexGuard<'_, Board> = self.board.lock().await;    //
        board.publish(BoardMessage {
            agent    : self.name.clone(),
            content  : response,
            timestamp: Utc::now().timestamp_millis() as u64,
        }).await;

        Ok(artifact)
    }

    async fn execute_next_move( &mut self, mission_id: &str, next_move: &NextMove ) -> Result<ObserverOutput> {
        // TODO

        let mut observer_output: ObserverOutput = ObserverOutput::default();
        
        match &next_move.action {

            Action::FileReader { path } => {

                println!("FileReader on path {}", path);

                let file_content: String = {
                    let board_guard =
                        self.board.lock().await;

                    board_guard
                        .registry
                        .observer_registry()
                        .file_reader
                        .execute(path)?
                };
                let prompt: String = format!(
r#"
OBJECTIVE
=========

{}

SOURCE IDENTIFIER
=================

{}

SOURCE CONTENT
==============
{}
"#,
                    next_move.intention,
                    path,
                    file_content,
                );

                println!("prompt {}", prompt);

                let registry: Arc<SystemRegistry> = {
                    let board_guard =
                        self.board.lock().await;

                    board_guard.registry.clone()
                };
                let dispatcher: Dispatcher = Dispatcher::new(registry);
                let response: String = dispatcher.execute(
                    "reading file...",
                    "worker-generic",
                    "sentinel-generic",
                    &prompt,
                    self.board.clone()
                ).await.unwrap_or_else(|e| {
                    format!("unable to generate response: {}", e)
                });

                println!("response {}", response);

                observer_output =
                    serde_json::from_str::<ObserverOutput>(
                        &response
                    )?;

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

        Ok(observer_output)
    }

}

fn extract_json(response: &str) -> Option<&str> {
    if let Some(pos) = response.rfind("</think>") {
        return Some(response[pos + "</think>".len()..].trim());
    }

    Some(response.trim())
}
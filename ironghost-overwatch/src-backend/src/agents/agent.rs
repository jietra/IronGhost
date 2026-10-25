// src/agents/agent.rs

use std::sync::Arc;
use tokio::sync::{
    Mutex,
    broadcast::{Receiver, error::RecvError},
    MutexGuard
};
use chrono::Utc;
use anyhow::{Result, anyhow};

use crate::core::{
    board::{Board, BoardEvent, BoardMessage},
    mission::Mission,
    mission_assessment::{AssessmentArtifact, AssessmentContext, NextAction, Action, ObserverOutput, StrategistOutput}
};
use crate::agents::agent_buffer::AgentBuffer;
use crate::llm::{
    llm::LLM,
    stream_forwarder::StreamForwarder
};
use crate::stores::source_store::FolderTree;
use crate::workers::dispatcher::Dispatcher;

pub struct Agent {
    pub name  : String,                             // agent's id
    pub buffer: AgentBuffer,                        // agent prompt to be sent to agent's llm service (last board publications since last agent's response)
    pub rx    : Receiver<BoardEvent>,               // agent receiver to receive board broadcasts
    pub llm   : LLM,                                // representation of agent's llm service
    pub board : Arc<Mutex<Board>>,                  // board the agent subscribes and publish to
}

impl Agent {
    pub fn new(
        name : &str,
        board: Arc<Mutex<Board>>,
        rx   : Receiver<BoardEvent>,
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

            Err(RecvError::Lagged(skipped)) => {
                eprintln!("[AGENT] WARNING: agent {} skipped {} messages", self.name, skipped);
                None
            }

            Err(RecvError::Closed) => {
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
                if let Err(e) =
                    self.on_command(mission_id, to, payload).await {
                        
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
            {
                let mut board: MutexGuard<'_, Board> = self.board.lock().await;    //
                
                board.publish(BoardMessage {
                    agent    : self.name.clone(),
                    content  : full_response,
                    timestamp: Utc::now().timestamp_millis() as u64,
                }).await;
            }
        }
    }

    async fn on_command( &mut self, mission_id: String, to: String, payload: String ) -> Result<()> {
        if self.name != to { return Ok(()); }

        println!("[AGENT] Agent {}, receiving command: {}", self.name, payload);

        match payload.as_str() {

            "!Initiate_Mission" => {
                // vvvvvvvvvvvvvvvv
                let budget: u32 = 2;
                // ^^^^^^^^^^^^^^^^
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
                
                self.launch_mission_assessment(budget).await?;

            }

            _ => {}
        }

        Ok(())

    }

    async fn refresh_source_tree( &mut self ) -> Result<()> {

        let source_tree: FolderTree = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;

            board
                .registry.observer_registry()
                .folder_explorer.execute()?
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

    // ----------------------------
    // mission assessment loop
    // ----------------------------

    async fn launch_mission_assessment( &mut self, budget: u32 ) -> Result<()> {

        let mission: Mission = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;

            board.current_mission()
        };
        
        let mut assessment_context: AssessmentContext = AssessmentContext {
            mission_prompt      : mission.prompt,
            source_tree_list    : mission.knowledge.source_tree_list,
            
            previous_assessment : None,
            observer_output     : None,
            iteration           : 0,
            budget              : budget,
        };

        for iteration in 0..budget {

            println!("[AGENT] mission iteration {}", iteration);

            let current_assessment: AssessmentArtifact =
                // send to agent
                self.run_assessment_round(&mut assessment_context).await?;

            match &current_assessment.next_move.action {

                Action::Finish => {
                    println!("[AGENT] mission assessment completed");
                    break;
                }

                _ => {
                    println!("[AGENT] executing next move {:?}", &current_assessment.next_move);

                    assessment_context.observer_output = Some(
                        // send to worker
                        self.execute_next_move( &current_assessment.next_move ).await?
                    );
                }                
            }

            //previous = artifact;
            assessment_context.previous_assessment = Some(current_assessment);
            assessment_context.iteration = iteration + 1;
        
        }

        Ok(())
    }

    // ----------------------------
    // send to agent LLM
    // ----------------------------

    async fn run_assessment_round ( &mut self, assessment_context: &mut AssessmentContext ) -> Result<AssessmentArtifact> {

        let prompt: String = assessment_context.to_prompt();

        let response: String =
            StreamForwarder::generate(
                &self.name,
                &prompt,
                &self.llm,
                self.board.clone(),
            ).await;

        // parse response
        println!("DEBUG response: {}", &response);

        let json_str: &str = extract_json(&response)
            .ok_or_else(|| anyhow!("unable to extract json"))?;

        // strategist output parsing
        let strategist_output: StrategistOutput =
            serde_json::from_str::<StrategistOutput>(
                json_str
            )?;
        
        println!("DEBUG strategist_output done");
        
        /*
        let intention: String   = strategist_output.next_action.intention;
        let target   : String   = strategist_output.next_action.target;
        let next_move: NextMove =
            match strategist_output.next_action.action.as_str() {
                "FileReader" => NextMove {
                    intention,
                    action: Action::FileReader { path: target }
                },

                // ...

                _ => NextMove::default(),
            };

        println!("next_move: {:?}", next_move);
        */

        let artifact: AssessmentArtifact = AssessmentArtifact {
            iteration   : assessment_context.iteration,
            analysis    : strategist_output.analysis,
            next_move   : strategist_output.next_move,
        };

        println!("artifact: {:?}", artifact);

        // publish
        {
            let mut board: MutexGuard<'_, Board> = self.board.lock().await;    //
            
            board.publish(BoardMessage {
                agent    : self.name.clone(),
                content  : response,
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;
        }
        
        Ok(artifact)

    }

    // ----------------------------
    // send to worker
    // ----------------------------

    async fn execute_next_move( &mut self, next_move: &NextAction ) -> Result<ObserverOutput> {
        // TODO: execute_next_move ((board), next_move)

        let mut observer_output: ObserverOutput = ObserverOutput::default();
        
        match &next_move.action {

            Action::FileReader => {
                // TODO: self.execute_file_reader((board), next_move).await

                let path: &String = &next_move.target;
                println!("FileReader on path {}", path);

                // execute FileReader tool
                let file_content: String = {
                    let board_guard: MutexGuard<'_, Board> = self.board.lock().await;

                    board_guard.registry
                        .observer_registry()
                        .file_reader.execute(path)?
                };

                // prompt for worker
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

                // send to FileReader worker
                let dispatcher: Dispatcher = {
                    let board_guard: MutexGuard<'_, Board> = self.board.lock().await;

                    board_guard.registry
                        .dispatcher()
                };

                let response: String = dispatcher.execute(
                    "reading file...",
                    "worker-generic",
                    "sentinel-generic",
                    &prompt,
                    self.board.clone()
                ).await
                .unwrap_or_else(|e| {
                    format!("unable to generate response: {}", e)
                });

                println!("response {}", response);

                // observer output
                observer_output =
                    serde_json::from_str::<ObserverOutput>(
                        &response
                    )?;

            }

            Action::WebSearcher => {
                // TODO
            }

            Action::WebReader => {
                // TODO
            }

            Action::AskForMoreContext => {
                // TODO
            }

            _ => {}
        }

        Ok(observer_output)
    }

}

// helper
fn extract_json(response: &str) -> Option<&str> {
    if let Some(pos) = response.rfind("</think>") {
        return Some(response[pos + "</think>".len()..].trim());
    }

    Some(response.trim())
}
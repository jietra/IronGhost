// src/agents/agent.rs

use std::sync::Arc;
use tokio::sync::{
    Mutex,
    broadcast::{Receiver, error::RecvError},
    MutexGuard
};
use chrono::Utc;
use anyhow::{Context, Result, anyhow};

use crate::core::{
    board::{Board, BoardEvent, BoardMessage}, mission::Mission, mission_assessment::{Action, AssessmentArtifact, AssessmentContext, NextAction, ObserverOutput, StrategistOutput}, mission_knowledge::{KnowledgeCard, MissionKnowledge}
};
use crate::knowledge_manager::{
    card_extractor::{ConceptRef, extract_concepts},
    card_prompt::build_card_upsert_prompt
};
use crate::agents::agent_buffer::AgentBuffer;
use crate::llm::{
    llm::LLM,
    stream_forwarder::StreamForwarder,
    response_parser::extract_json,
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
                
                let budget: u32 = 10;
                // ^^^^^^^^^^^^^^^^
                // we set a budget = max iterations in assessment loop
                // TODO: put budget in configuration parameters or call args, or default
                // OPTIM: fine-grain budgeting with websearch budget, sub-mission budget (depth), next_move batch budget etc.
                //
                
                println!("[AGENT] Agent {}, initiating mission: {}", self.name, mission_id);
                
                self.refresh_source_tree().await?;
                
                self.execute_mission_assessment(budget).await?;

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

    async fn execute_mission_assessment( &mut self, budget: u32 ) -> Result<()> {
        
        let mut pending_observation: Option<ObserverOutput> = None;

        for iteration in 0..budget {

            let is_last_iteration: bool = iteration + 1 >= budget;

            // 1. Build assessment context
            let mission: Mission = {
                let board: MutexGuard<'_, Board> = self.board.lock().await;
                board.current_mission()
            };

            let assessment_context: AssessmentContext = AssessmentContext {
                mission             : mission,
                observer_output     : pending_observation.clone(),
                iteration           : iteration,
                budget              : budget,
            };

            println!("[AGENT] mission iteration {}", iteration);

            // 2. Update assessment: strategist consumes the pending observation.
            let mut current_assessment: AssessmentArtifact =
                // send to agent
                self.run_assessment_round(&assessment_context).await?;

/*
            // 3. Prepare knowledge updates from the same observation.
            let updated_cards: Vec<KnowledgeCard> = match pending_observation.as_ref() {
                Some(observer_output) => {
                    self.prepare_knowledge_updates(
                        &assessment_context.mission.prompt,
                        &current_assessment.assessment_report,
                        observer_output,
                    )
                    .await?
                }

                None => Vec::new(),
            };
*/

            // 4. Persist a consistent knowledge snapshot: update board with assessment result AND update knowledge (keep it coherent)
            {
                let mut board: MutexGuard<'_, Board> = self.board.lock().await;
                let knowledge: &mut MissionKnowledge = &mut board.mission.knowledge;

                // cards update
//                for card in updated_cards { knowledge.upsert_card(card)?; }

                // assessment update
                knowledge.current_assessment = Some(current_assessment.clone());
            }
            
            // 5. Execute next move

            if is_last_iteration {
                current_assessment.next_move = NextAction {
                    intention: "Assessment budget exhausted.".into(),
                    action: Action::Finish,
                    target: String::new(),
                };
            }

            match &current_assessment.next_move.action {

                Action::Finish => {
                    println!("[AGENT] mission assessment completed");
                    break;
                }

                _ => {
                    println!("[AGENT] executing next move {:?}", &current_assessment.next_move);

                    pending_observation = Some(
                        // send to worker
                        self.execute_next_move( &current_assessment.next_move ).await?
                    );
                }                
            }
            
        }

        Ok(())
    }

    // ----------------------------
    // send to agent LLM
    // ----------------------------

    async fn run_assessment_round ( &mut self, assessment_context: &AssessmentContext ) -> Result<AssessmentArtifact> {

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

        let artifact: AssessmentArtifact = AssessmentArtifact {
            iteration           : assessment_context.iteration,
            assessment_report   : strategist_output.assessment_report,
            next_move           : strategist_output.next_move,
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
        
        match &next_move.action {

            Action::FileReader => {
                self.execute_file_reader(next_move).await
            }

            Action::WebSearcher => {
                // TODO
                Err(anyhow!("WebSearcher is not implemented"))
            }

            Action::WebReader => {
                // TODO
                Err(anyhow!("WebReader is not implemented"))
            }

            Action::AskForMoreContext => {
                // TODO
                Err(anyhow!("AskForMoreContext is not implemented"))
            }

            Action::FolderExplorer => {
                Err(anyhow!("FolderExplorer is not implemented"))
            }

            Action::Finish => {
                Err(anyhow!(
                    "Finish must not be executed as an observation action"
                ))
            }

            //_ => {Err(anyhow!("Requested action not found"))}
        }

    }

    async fn execute_file_reader( &self, next_move: &NextAction ) -> Result<ObserverOutput> {
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
        .map_err(|error| {
            anyhow!(
                "observation worker failed for source {:?}: {}",
                path,
                error
            )
        })?;

        println!("response {}", response);

        // observer output
        let json: &str = extract_json(&response)
            .ok_or_else(|| {
                anyhow!(
                    "unable to extract ObserverOutput JSON for source {:?}",
                    path
                )
            })?;

        let observer_output: ObserverOutput = serde_json::from_str::<ObserverOutput>(json)
            .with_context(|| {
                format!(
                    "failed to parse ObserverOutput for source {:?}",
                    path
                )
            })?;

        // avoid inconsistency in source_id
        if observer_output.source_id != *path {
            return Err(anyhow!(
                "observer returned source_id {:?}, expected {:?}",
                observer_output.source_id,
                path
            ));
        }
        
        {
            let mut board_guard = self.board.lock().await;

            board_guard
                .mission
                .knowledge
                .source_visited
                .insert(next_move.target.clone());
        }

        Ok(observer_output)
    }

    // ----------------------------
    // update mission knowledge
    // ----------------------------

    async fn prepare_knowledge_updates(
        &self,
        mission_prompt : &str,
        assessment     : &str,
        observer_output: &ObserverOutput,
    ) -> Result<Vec<KnowledgeCard>> {
        let concepts: Vec<ConceptRef> = extract_concepts(assessment);
        let mut updated_cards: Vec<KnowledgeCard> = Vec::with_capacity(concepts.len());
        
        for concept in concepts {
            let updated_card: KnowledgeCard = self.generate_updated_card(
                mission_prompt,
                &concept,
                observer_output
            ).await?;

            updated_cards.push(updated_card);
        }

        Ok(updated_cards)
    }

    async fn generate_updated_card(
        &self,
        mission_prompt : &str,
        concept        : &ConceptRef,
        observer_output: &ObserverOutput,
    ) -> Result<KnowledgeCard> {
        let current_card: Option<KnowledgeCard> = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;

            board
                .mission
                .knowledge
                .get_card_cloned(&concept.id)
        };

        let prompt: String = build_card_upsert_prompt(
            mission_prompt,
            concept,
            current_card.as_ref(),
            observer_output,
        )?;

        let dispatcher: Dispatcher = {
            let board: MutexGuard<'_, Board> = self.board.lock().await;
            board.registry.dispatcher()
        };

        let response: String = dispatcher
            .execute(
                &format!("upsert knowledge card {}", concept.id),
                "worker-knowledge",
                "sentinel-knowledge",
                &prompt,
                self.board.clone(),
            )
            .await
            .map_err(|e| anyhow!(
                "failed to update KnowledgeCard {:?}: {}",
                concept.id,
                e
            ))?;

        let json: &str = extract_json(&response)
            .ok_or_else(|| {
                anyhow!(
                    "unable to extract KnowledgeCard JSON for {:?}",
                    concept.id
                )
            })?;
            
        //let mut updated_card: KnowledgeCard =
        let updated_card: KnowledgeCard =
            serde_json::from_str(json)
                .with_context(|| {
                    format!(
                        "failed to parse KnowledgeCard {:?}",
                        concept.id
                    )
                })?;

        if updated_card.id != concept.id {
            return Err(anyhow!(
                "knowledge worker returned card id {:?}, expected {:?}",
                updated_card.id,
                concept.id,
            ));
        }

        if updated_card.title != concept.title {
            return Err(anyhow!(
                "knowledge worker returned title {:?}, expected {:?}",
                updated_card.title,
                concept.title
            ));
        }

        /*
        if !updated_card.references.contains(&observer_output.source_id) {
            updated_card
                .references
                .push(observer_output.source_id.clone());
        }

        updated_card.references.sort();
        updated_card.references.dedup();
        */

        /*
        {
            let mut board: MutexGuard<'_, Board> = self.board.lock().await;

            let previous: Option<KnowledgeCard> =
                board
                    .mission
                    .knowledge
                    .upsert_card(updated_card)?;
            
            match previous {
                Some(old_card) => {
                    println!(
                        "Updated card {} from status {:?}",
                        old_card.id,
                        old_card.status
                    );
                }
                None => {
                    println!("Created new card");
                }
            }
        }
        */

        Ok(updated_card)
    }

}
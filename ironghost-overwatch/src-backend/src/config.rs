// src/config.rs

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::broadcast::Sender;
use crate::core::board::Board;
use crate::core::board::BoardEvent;
use crate::agents::agent::Agent;
use crate::agents::agent_llm::AgentLLM;

pub struct SystemRegistry {
    // AI agents
    agents   : HashMap<String, String>,     // reasoning agents (AI)
    workers  : HashMap<String, String>,     // specialized agents (AI, bounded memory)
    sentinels: HashMap<String, String>,     // sentinels (stateless)

    // Deterministical tools
    observers: HashMap<String, String>,     // passive tools: sensors, recon (deterministic)
    executors: HashMap<String, String>,     // active tools : actuators (deterministic) (only with isolated environments)
}

impl SystemRegistry {
    pub fn new() -> Self {
        // reasoning agents
        let mut agents: HashMap<String, String> = HashMap::new();

        agents.insert(
            "strategist".into(),
            "/tmp/agent_strategist.sock".into(),
        );

        agents.insert(
            "generic".into(),
            "/tmp/agent_generic.sock".into(),
        );

        // specialized agents
        let mut workers: HashMap<String, String> = HashMap::new();

        workers.insert(
            "coder".into(),
            "/tmp/agent_coder.sock".into(),
        );

        // sentinels
        let mut sentinels: HashMap<String, String> = HashMap::new();

        sentinels.insert(
            "sentinel-code".into(),
            "/tmp/sentinel_code.sock".into(),
        );

        // observers
        let observers: HashMap<String, String> = HashMap::new();

        // executors
        let executors: HashMap<String, String> = HashMap::new();

        Self {
            agents,
            workers,
            sentinels,
            observers,
            executors,
        }
    }

    pub fn resolve_worker(&self, name: &str) -> Option<&str> {
        self.workers
            .get(name)
            .map(String::as_str)
    }

    pub fn resolve_sentinel(&self,name: &str) -> Option<&str> {
        self.sentinels
            .get(name)
            .map(String::as_str)
    }

    pub async fn spawn_agents(&self, board: Arc<Mutex<Board>>) {

        let tx: Sender<BoardEvent> = {
            let board_guard: tokio::sync::MutexGuard<'_, Board> =
                board.lock().await;
            board_guard.tx.clone()
        };

        for (name, socket) in &self.agents
        {
            let agent: Agent = Agent::new(
                name,
                board.clone(),
                tx.subscribe(),
                AgentLLM::new(socket),
            );

            agent.spawn();
        }
    }

}


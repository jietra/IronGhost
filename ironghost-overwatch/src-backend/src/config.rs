// src/config.rs

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{
    Mutex,
    broadcast::Sender
};
use crate::core::board::{
    Board,
    BoardEvent
};
use crate::agents::{
    agent::Agent,
    agent_llm::AgentLLM
};
use crate::observers::{
    file_reader::FileReader,
    folder_explorer::FolderExplorer,
    web_searcher::WebSearcher,
    web_searcher::BraveWebSearcher
};
use crate::artifact_store::artifact_store::{
    FilesystemArtifactStore
};

#[derive(Debug, Clone)]
pub struct Config {
    pub artifact_store_root : String,
    pub brave_api_url       : String,
    pub brave_api_key       : String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        //dotenvy::dotenv().ok();

        Ok(Self {
            artifact_store_root : std::env::var("ARTIFACT_STORE_ROOT")?,
            brave_api_url       : std::env::var("BRAVE_API_URL")?,
            brave_api_key       : std::env::var("BRAVE_API_KEY")?,
        })
    }
}

pub struct ObserverRegistry {
    pub file_reader     : FileReader,
    pub folder_explorer : FolderExplorer,
    pub web_searcher    : Arc<dyn WebSearcher>,
}

//
// =========================
// System Registry
// =========================
// bootstrap system with a system registry

pub struct SystemRegistry {
    config        : Arc<Config>,                // environment variables

    // AI agents
    agents        : HashMap<String, String>,    // reasoning agents (AI)
    workers       : HashMap<String, String>,    // specialized agents (AI, bounded memory)
    sentinels     : HashMap<String, String>,    // sentinels (stateless)

    // Deterministical tools
    observers     : ObserverRegistry,           // passive tools: sensors, recon (deterministic)
    executors     : HashMap<String, String>,    // active tools : actuators (deterministic) (only with isolated environments)

    // Artifact store
    artifact_store: Arc<FilesystemArtifactStore>,   // file system artifact store only for now
}

impl SystemRegistry {

    pub fn config(&self) -> &Config {
        self.config.as_ref()
    }

    pub fn new() -> anyhow::Result<Self> {

        let config: Arc<Config> = Arc::new(Config::load()?);

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
        let observers: ObserverRegistry = ObserverRegistry {
                file_reader     : FileReader,
                folder_explorer : FolderExplorer,
                web_searcher    : Arc::new(
                    BraveWebSearcher::new(config.clone())
                ),
            };

        // executors
        let mut executors: HashMap<String, String> = HashMap::new();

        executors.insert(
            "executor_orchestrator".into(),
            "/tmp/executor.sock".into(),
        );

        // artifact store
        let artifact_store: Arc<FilesystemArtifactStore> = Arc::new(
            FilesystemArtifactStore::new(config.artifact_store_root.clone())
        );

        Ok(Self {
            config,
            agents,
            workers,
            sentinels,
            observers,
            executors,
            artifact_store,
        })
    }

    // workers and sentinels are resolved (they generic services)
    pub fn resolve_worker(&self, name: &str) -> Option<&str> {
        self.workers.get( name ).map(String::as_str)
    }

    pub fn resolve_sentinel(&self,name: &str) -> Option<&str> {
        self.sentinels.get( name ).map(String::as_str)
    }

    // cognitive agents are spawned and linked to a Board (they are persistent services)
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


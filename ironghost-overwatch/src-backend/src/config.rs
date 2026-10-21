// src/config.rs

use std::{
    fs,
    path::Path,
    sync::Arc,
    collections::HashMap,
};
use tokio::sync::{
    Mutex,
    broadcast::Sender
};
use serde::Deserialize;
use anyhow::bail;

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
    web_searcher::BraveWebSearcher,
    web_reader::WebReader,
};
use crate::stores::{
    source_store::SourceStore,
    artifact_store::FilesystemArtifactStore,
};

#[derive(Debug, Deserialize)]
pub struct YamlConfig {
    pub artifact_store  : ArtifactStoreConfig,
    pub source_store    : SourceStoreConfig,
    pub agents          : HashMap<String, SocketConfig>,
    pub workers         : HashMap<String, SocketConfig>,
    pub sentinels       : HashMap<String, SocketConfig>,
    pub observers       : ObserverConfig,
    pub executors       : HashMap<String, SocketConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactStoreConfig {
    pub root: String,
}

#[derive(Debug, Deserialize)]
pub struct SourceStoreConfig {
    pub root        : String,
    pub max_size_mb : u64,
}

#[derive(Debug, Deserialize)]
pub struct SocketConfig {
    pub socket: String,
}

#[derive(Debug, Deserialize)]
pub struct ObserverConfig {
    pub web_search: WebSearchConfig,
}

#[derive(Debug, Deserialize)]
pub struct WebSearchConfig {
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub brave_api_url       : String,
    pub brave_api_key       : String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {

        Ok(Self {
            brave_api_url       : std::env::var("BRAVE_API_URL")?,
            brave_api_key       : std::env::var("BRAVE_API_KEY")?,
        })
    }
}

pub struct ObserverRegistry {
    pub file_reader     : FileReader,
    pub folder_explorer : FolderExplorer,
    pub web_searcher    : Arc<dyn WebSearcher>,
    pub web_reader      : WebReader,
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

    // Stores
    artifact_store: Arc<FilesystemArtifactStore>,   // file system artifact store only for now
    source_store  : Arc<SourceStore>,
}

impl SystemRegistry {

    pub fn config(&self) -> &Config {self.config.as_ref()}

    pub fn from_yaml( path: impl AsRef<Path> ) -> anyhow::Result<Self> {

        let yaml    : String        = fs::read_to_string(      path    )?;
        let cfg     : YamlConfig    = serde_yaml::from_str(    &yaml   )?;

        // config
        let config  : Arc<Config>   = Arc::new(Config::load()?);

        // artifact store
        let artifact_store: Arc<FilesystemArtifactStore> = Arc::new(
            FilesystemArtifactStore::new(cfg.artifact_store.root)
        );

        // source store
        let source_store: Arc<SourceStore> = Arc::new(
            SourceStore::new(cfg.source_store.root, cfg.source_store.max_size_mb * 1024 * 1024)?
        );

        // observers
        let web_searcher = match cfg.observers.web_search.provider.as_str() {
            "brave" => {
                Arc::new(
                    BraveWebSearcher::new(config.clone())
                )
            }
            provider => bail!("[CONFIG]: Unknown web search provider: {}", provider)
        };
        let observers: ObserverRegistry = ObserverRegistry {
            file_reader     : FileReader::new(source_store.clone()),
            folder_explorer : FolderExplorer::new(source_store.clone()),
            web_searcher    : web_searcher,
            web_reader      : WebReader::new(),
        };

        Ok(Self {
            config,
            agents      : cfg.agents.into_iter().map(|(k,v)| (k, v.socket)).collect(),
            workers     : cfg.workers.into_iter().map(|(k,v)| (k, v.socket)).collect(),
            sentinels   : cfg.sentinels.into_iter().map(|(k,v)| (k, v.socket)).collect(),
            observers,
            executors   : cfg.executors.into_iter().map(|(k,v)| (k, v.socket)).collect(),
            artifact_store,
            source_store,
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

    // other getters
    pub fn observer_registry( &self ) -> &ObserverRegistry {
        &self.observers
    }

    pub fn artifact_store( &self ) -> Arc<FilesystemArtifactStore> {
        self.artifact_store.clone()
    }

    pub fn source_store( &self ) -> Arc<SourceStore> {
        self.source_store.clone()
    }

}


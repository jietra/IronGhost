// src/config.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::core::board::Board;
use crate::agents::agent::Agent;
use crate::agents::agent_llm::AgentLLM;

pub struct AppConfig;

impl AppConfig {
    pub async fn default_agents(board: Arc<Mutex<Board>>) -> Vec<Agent> {
        // create broadcast channel
        // cannot use board.tx directly since board is a Arc<tokio::Mutex<Board>>.
        let tx = {
            // unlock mutex first to access actual board (board is a shared pointer to a mutex)
            // this block {...} allows a direct freeing of the mutex (the gard is immediately destroyed).
            // otherwise, the wait would last till end of scope
            let board_guard = board.lock().await;
            board_guard.tx.clone()
        };

        vec![
            Agent::new(
                "Strategist",
                board.clone(),
                tx.subscribe(),
                AgentLLM::new("/tmp/agent_strategist.sock"),
            ),
            Agent::new(
                "Generic",
                board.clone(),
                tx.subscribe(),
                AgentLLM::new("/tmp/agent_generic.sock"),
            ),
        ]
    }
}
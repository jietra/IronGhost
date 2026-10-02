// src/bin/main.rs

use backend::{
    config::AppConfig,
    core::board::Board,
    engine::runner::MissionRunner,
    engine::registry::AgentRegistry,
    ui::websocket::run_websocket_ui,
};
use std::sync::Arc;         // share an object (Board) between multiple tasks
use tokio::sync::Mutex;     // mutex async (block tasks, not threads)

#[tokio::main]  // macro-attribute launching Tokio runtime with main() as unique entry point (for async: futures...)
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. init board
    let board = Arc::new(Mutex::new(Board::new()));
    println!("[MAIN] new board initiated");

    // 2. create and run agents (via spawn)
    let agents = AppConfig::default_agents(board.clone()).await;
    for agent in agents {
        agent.spawn();
    }
    println!("[MAIN] agents spawned and running");

    // 3. launch Runner DAG
    let registry = Arc::new(AgentRegistry::new());
    let runner = MissionRunner::new(board.clone(), registry);
    runner.spawn_listener(); // Écoute le Board et lance le DAG quand la Mission change

    // 4. launch UI
    tokio::spawn(run_websocket_ui("127.0.0.1:9000", board.clone()));

    //tokio::spawn(websocket_ui_server(board.clone()));

    println!("[MAIN] system ready. Waiting for agents...");
    tokio::signal::ctrl_c().await?;
    println!("[MAIN] shutting down IronGhost...");

    Ok(())
}
// src/bin/main.rs

use backend::{
    config::SystemRegistry,
    core::board::Board,
    engine::runner::MissionRunner,
    ui::websocket::run_websocket_ui,
};
use std::sync::Arc;         // share an object (Board) between multiple tasks
use tokio::sync::Mutex;     // mutex async (block tasks, not threads)

#[tokio::main]  // macro-attribute launching Tokio runtime with main() as unique entry point (for async: futures...)
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    //dotenvy::dotenv().ok();

    // 1. init board
    let board: Arc<Mutex<Board>> = Arc::new(
        Mutex::new(Board::new())
    );
    println!("[MAIN] new board initialized");

    // 2. init system registry
    let registry: Arc<SystemRegistry> = Arc::new(
        SystemRegistry::new()
    );
    println!("[MAIN] system registry initialized");

    // 3. spawn reasoning agents
    registry.spawn_agents(board.clone()).await;
    println!("[MAIN] agents spawned and running");

    // 4. launch Runner DAG
    let runner: MissionRunner = MissionRunner::new(
        board.clone(),
        registry.clone()
    );
    runner.spawn_listener(); // Listens to Board and launch DAG when Mission changes

    // 5. launch UI
    tokio::spawn(run_websocket_ui("127.0.0.1:9000", board.clone()));

    println!("[MAIN] system ready. Waiting for agents...");

    // 6. shutdown
    tokio::signal::ctrl_c().await?;
    println!("[MAIN] shutting down IronGhost...");

    Ok(())
}
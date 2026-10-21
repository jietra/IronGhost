// src/bin/main.rs

use backend::{
    agents::system_agent::SystemAgent, config::SystemRegistry, core::board::{Board, BoardEvent}, engine::runner::MissionRunner, ui::websocket::run_websocket_ui,
};
use std::sync::Arc;         // share an object (Board) between multiple tasks
use tokio::sync::{Mutex, broadcast::Sender};     // mutex async (block tasks, not threads)

#[tokio::main]  // macro-attribute launching Tokio runtime with main() as unique entry point (for async: futures...)
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    dotenvy::dotenv().ok();

    // 1. init system registry
    let registry: Arc<SystemRegistry> = Arc::new(
        SystemRegistry::from_yaml("../../ironghost.yaml")?
    );
    println!("[MAIN] system registry initialized");
    //println!("cwd={}", std::env::current_dir()?.display());

    // 2. init board
    let board: Arc<Mutex<Board>> = Arc::new(
        Mutex::new(Board::new(
            registry.clone()    //.artifact_store()
        ))
    );
    println!("[MAIN] new board initialized");

    let tx: Sender<BoardEvent> = {
        let board_guard: tokio::sync::MutexGuard<'_, Board> =
            board.lock().await;
        board_guard.tx.clone()
    };

    // 3. spawn system agent
    let system_agent: SystemAgent = SystemAgent::new(board.clone(), tx.subscribe());
    let _ = system_agent.spawn();

    // 4. spawn reasoning agents
    registry.spawn_agents(board.clone()).await;
    println!("[MAIN] agents spawned and running");

    // 5. launch Runner DAG
    let runner: MissionRunner = MissionRunner::new(
        board.clone(),
        registry.clone()
    );
    runner.spawn_listener(); // Listens to Board and launch DAG when Mission changes

    // 6. launch UI
    tokio::spawn(run_websocket_ui("127.0.0.1:9000", board.clone()));

    println!("[MAIN] system ready. Waiting for agents...");

    // 7. shutdown
    tokio::signal::ctrl_c().await?;
    println!("[MAIN] shutting down IronGhost...");

    Ok(())
}
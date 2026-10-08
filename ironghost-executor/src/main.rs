// src/main.rs

pub mod environments;
pub mod executors;
pub mod models;
pub mod orchestrator;
pub mod server;

use crate::orchestrator::ExecutorOrchestrator;
use crate::server::ExecutorServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let orchestrator = ExecutorOrchestrator::new();

    ExecutorServer::new(
        "/tmp/executor.sock",
        orchestrator,
    )
    .run()
    .await
}
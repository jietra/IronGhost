// src/server.rs

use std::path::Path;
use tokio::net::{UnixListener,UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::orchestrator::ExecutorOrchestrator;
use crate::models::{ExecutionRequest, ExecutionVerdict};


pub struct ExecutorServer {
    pub socket_path : String,
    pub orchestrator: ExecutorOrchestrator,
}

impl ExecutorServer {
    pub fn new(
        socket_path : &str,
        orchestrator: ExecutorOrchestrator,
    ) -> Self {
        Self {
            socket_path : socket_path.to_string(),
            orchestrator,
        }
    }

    pub async fn run(
        &self,
    ) -> anyhow::Result<()> {

        if Path::new( &self.socket_path ).exists() {
            std::fs::remove_file( &self.socket_path )?;
        }

        let listener: UnixListener = UnixListener::bind( &self.socket_path )?;

        println!("[EXECUTOR] listening on {}", &self.socket_path);

        loop {
            let (stream, _) = listener.accept().await?;

            let orchestrator: ExecutorOrchestrator = self.orchestrator.clone();

            tokio::spawn(async move {
                if let Err(err) = Self::handle_connection( stream, orchestrator ).await
                {
                    eprintln!("[EXECUTOR] {}", err);
                }

            });
        }
    }

    pub async fn handle_connection(
        mut stream  : UnixStream,
        orchestrator: ExecutorOrchestrator,
    ) -> anyhow::Result<()> {

        let mut buffer: Vec<u8> = Vec::new();

        stream.read_to_end(&mut buffer).await?;

        let request: ExecutionRequest =
            serde_json::from_slice( &buffer )?;

        let verdict: ExecutionVerdict  = orchestrator.execute( request ).await?;

        let response: Vec<u8> = serde_json::to_vec( &verdict )?;

        stream.write_all( &response ).await?;

        Ok(())
    }

}
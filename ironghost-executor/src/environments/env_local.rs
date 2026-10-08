// src/environments/env_local.rs

/// WARNING: This environment is not safe and is for testing purposes ONLY. It is not secure and should not be used in production.
/// Production executors have to be deployed in secure environments (e.g. micro-VM).
/// Excutors can run untrusted and potentially armfull code, so they must be isolated from the rest of the system and have limited access to resources.

use async_trait::async_trait;
use crate::environments::environment::{Environment};
use crate::environments::environment_handle::EnvironmentHandle;
use crate::models::ExecutionVerdict;
use tokio::process::Command;

#[derive(Debug)]
pub struct LocalEnvironment {
    handle: EnvironmentHandle,
}

impl LocalEnvironment {
    pub fn new() -> Self {
        Self {
            handle: EnvironmentHandle {
                id  : "local".into(),
                name: "local".into(),
                ip  : "127.0.0.1".into(),
            },
        }
    }
}

#[async_trait]
impl Environment for LocalEnvironment {

    fn handle(&self) -> &EnvironmentHandle {
        &self.handle
    }

    async fn upload_file(
        &self,
        path    : &str,
        content : &str,
    ) -> anyhow::Result<()> {

        tokio::fs::write(
            path,
            content,
        )
        .await?;
    
        Ok(())
    }

    async fn execute_command(
        &self,
        command : &str,
    ) -> anyhow::Result<ExecutionVerdict> {

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;

        Ok(ExecutionVerdict {
            success     : output.status.success(),
            stdout      : String::from_utf8_lossy( &output.stdout ).to_string(),
            stderr      : String::from_utf8_lossy( &output.stderr ).to_string(),
            exit_code   : output.status.code().unwrap_or(-1),

            //duration_ms         : 0,
            //working_directory   : "cwd".into(),
        })
    }

}
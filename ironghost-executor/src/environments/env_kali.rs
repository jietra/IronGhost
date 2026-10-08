// src/environments/env_kali_utm.rs

use async_trait::async_trait;
use tokio::process::Command;
use std::process::Output;

use crate::environments::environment::Environment;
use crate::environments::environment_handle::EnvironmentHandle;
use crate::models::ExecutionVerdict;

#[derive(Debug)]
pub struct KaliEnvironment {
    handle: EnvironmentHandle,
}

impl KaliEnvironment {
    pub fn new() -> Self {
        Self {
            handle: EnvironmentHandle {
                id  : "kali".into(),
                name: "ironghost-worker".into(),
                ip  : "ironghost-worker".into(),
            },
        }
    }
}

#[async_trait]
impl Environment for KaliEnvironment {

    fn handle(&self) -> &EnvironmentHandle {
        &self.handle
    }

    async fn upload_file(
        &self,
        path    : &str,
        content : &str,
    ) -> anyhow::Result<()> {

        let tmp_file_local: String = format!("/tmp/{}", uuid::Uuid::new_v4());

        tokio::fs::write(
            &tmp_file_local,
            content,
        )
        .await?;

        let output: Output = Command::new("scp")
            .arg(&tmp_file_local)
            .arg(format!("{}:{}", self.handle.ip, path))
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "scp failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let _ = tokio::fs::remove_file( &tmp_file_local ).await;
    
        Ok(())
    }

    async fn execute_command(
        &self,
        command : &str,
    ) -> anyhow::Result<ExecutionVerdict> {

        println!(
            "[KALI] Executing '{}' on {}",
            command,
            self.handle.ip,
        );
        
        let output: Output = Command::new("ssh")
            .arg(&self.handle.ip)
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
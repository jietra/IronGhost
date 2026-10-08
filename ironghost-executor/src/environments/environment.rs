// src/environments/environment.rs

use async_trait::async_trait;

use crate::environments::environment_handle::EnvironmentHandle;
use crate::models::ExecutionVerdict;

#[async_trait]
pub trait Environment:
    Send + Sync + std::fmt::Debug {

    fn handle(&self) -> &EnvironmentHandle;

    async fn upload_file(
        &self,
        path: &str,
        content: &str,
    ) -> anyhow::Result<()>;

    async fn execute_command(
        &self,
        command: &str,
    ) -> anyhow::Result<ExecutionVerdict>;

}
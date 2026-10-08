// src/executors/runtime_executor.rs

use async_trait::async_trait;
use crate::models::{
    ExecutionRequest,
    ExecutionVerdict,
};
use crate::environments::environment::Environment;

#[async_trait]
pub trait RuntimeExecutor:
    Send + Sync + std::fmt::Debug {

    async fn execute(
        &self,
        env: &dyn Environment,
        req: ExecutionRequest,
    ) -> anyhow::Result<ExecutionVerdict>;

}
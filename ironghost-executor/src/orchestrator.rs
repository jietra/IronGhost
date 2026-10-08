// src/orchestrator.rs

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::anyhow;

use crate::executors::{
    python::PythonExecutor,
    runtime_executor::RuntimeExecutor,
};
use crate::models::{
    ExecutionRequest,
    ExecutionVerdict,
    RuntimeKind,
};
use crate::environments::{
    environment::Environment,
    environment_manager::EnvironmentManager,
};

#[derive(Debug, Clone)]
pub struct ExecutorOrchestrator {
    environment_manager : EnvironmentManager,
    runtimes            : HashMap< RuntimeKind, Arc<dyn RuntimeExecutor> >,
}

impl ExecutorOrchestrator {
    pub fn new() -> Self {
        let mut runtimes: HashMap< RuntimeKind, Arc<dyn RuntimeExecutor> > = HashMap::new();
        
        runtimes.insert(
            RuntimeKind::Python,
            Arc::new(PythonExecutor),
        );

        Self { 
            environment_manager : EnvironmentManager::new(),
            runtimes            : runtimes,
        }
    }

    pub async fn execute(
        &self,
        req: ExecutionRequest,
    ) -> anyhow::Result<ExecutionVerdict> {

        let env: Arc<dyn Environment> = self.environment_manager.acquire(
            req.environment.clone()
        ).await?;

        let executor: &Arc<dyn RuntimeExecutor> =
            self.runtimes.get(
                    &req.runtime
                ).ok_or_else(|| anyhow!(
                    "[ORCHESTRATOR] Runtime not found"
                )
            )?;

        executor.execute(env.as_ref(), req).await

    }
}
// src/executors/python.rs

use crate::executors::runtime_executor::RuntimeExecutor;
use crate::models::{
    ExecutionRequest,
    ExecutionVerdict,
    //RuntimeKind
};
use crate::environments::environment::Environment;

#[derive(Debug)]
pub struct PythonExecutor;

#[async_trait::async_trait]
impl RuntimeExecutor for PythonExecutor {

    async fn execute(
        &self,
        env: &dyn Environment,
        req: ExecutionRequest,
    ) -> anyhow::Result<ExecutionVerdict> {

        //match req.runtime {

            //RuntimeKind::Python => {
                let path: String = format!(
                    "/tmp/ironghost_{}.py",
                    uuid::Uuid::new_v4()
                );

                env.upload_file(
                    &path,
                    &req.source
                ).await?;

                let verdict: ExecutionVerdict = env.execute_command(&format!(
                    "python3 {}",
                    path
                )).await?;

                //let _ = tokio::fs::remove_file( &file ).await;

                Ok(verdict)
            //}
/* 
            _ => {
                Ok(
                    ExecutionVerdict {
                        success  : false,
                        stdout   : String::new(),
                        stderr   : format!(
                            "[EXECUTOR] Unsupported runtime: {:?}",
                            req.runtime
                        ),
                        exit_code: -1,
                    }
                )
            }
*/
        //}
    }
}
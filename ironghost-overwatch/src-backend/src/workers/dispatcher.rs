// src/engine/dispatcher.rs

use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::board::{Board, BoardMessage};
use crate::core::mission::MissionNode;
use crate::config::SystemRegistry;
use crate::llm::{llm::LLM, stream_forwarder::StreamForwarder};

// TODO: to be set in config params
const MAX_RETRIES: usize = 3;
// --------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentinelVerdict {
    pub compliant           : bool,
    pub score               : u8,
    pub missing_requirements: Vec<String>,
}

impl SentinelVerdict {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn to_feedback(&self) -> String {
        let requirements: String =
            self.missing_requirements
                .iter()
                .map(|r| format!("- {}", r))
                .collect::<Vec<_>>()
                .join("\n");
        
        format!(
            "Compliance: {}\n\
            (Score: {}/10)\n\n\
            Missing requirements:\n{}",
            self.compliant,
            self.score,
            requirements,
        )
    }
}

pub struct TaskExecutionContext {
    pub task_objective  : String,
    pub trial           : usize,
    pub previous_output : Option<String>,
    pub feedback        : Option<SentinelVerdict>,
}

impl TaskExecutionContext {
    pub fn to_worker_prompt(&self) -> String {
        let mut prompt: String =
            format!(
                "[TASK OBJECTIVE]\n\n{}\n",
                self.task_objective
            );

        if let Some(previous_output) = &self.previous_output {
            prompt.push_str(
                &format!(
                    "\n[PREVIOUS GENERATED OUTPUT]\n\n{}\n",
                    previous_output
                )
            );
        }

        if let Some(verdict) = &self.feedback {
            prompt.push_str(
                &format!(
                    "\n[FEEDBACK ON PREVIOUS OUTPUT]\n\n{}\n",
                    verdict.to_feedback()
                )
            );
        }

        prompt
    }

    pub fn to_sentinel_prompt(
        &self,
        generated_output: &str,
    ) -> String {

        format!(
            "[TASK OBJECTIVE]\n\n{}\n\n\
             [GENERATED OUTPUT]\n\n{}",
            self.task_objective,
            generated_output
        )
    }
}

pub struct Dispatcher {
    registry: Arc<SystemRegistry>,
}

impl Dispatcher {
    pub fn new(registry: Arc<SystemRegistry>) -> Self {
        Self { registry }
    }

    pub async fn execute_node(
        &self,
        node: &MissionNode,
        board: Arc<Mutex<Board>>,
    ) -> Result<String, String> {

        let worker: &str = node.worker.as_deref().unwrap_or("generic");
        let task = &node.title;
        let sentinel = "sentinel-code";
        let instruction = &node.description.clone().unwrap_or_else(|| node.title.clone());

        self.execute(task, worker, sentinel, instruction, board).await
        
    }

    pub async fn execute(
        &self,
        task        : &str,
        worker      : &str,
        sentinel    : &str,
        instruction : &str,
        board       : Arc<Mutex<Board>>,
    ) -> Result<String, String> {

        // TODO: worker automatic fallback to generic worker
        // TODO: link sentinel to worker (e.g. in the yaml, add a sentinel line per worker)

        // worker
        let socket_path: &str = self.registry
            .resolve_worker(worker)
            .ok_or_else(|| format!("[DISPATCHER] unknown worker {:?}", worker))?;
        let client: LLM = LLM::new(socket_path);
        println!("[DISPATCHER] dispatching task to {}", socket_path);

        // sentinel
        let sentinel_socket: &str = self.registry
            .resolve_sentinel(sentinel)
            .ok_or_else(|| format!("[DISPATCHER] sentinel not found for worker {:?}", worker))?;
        let sentinel_client: LLM = LLM::new(sentinel_socket);

        // initialize task execution context
        let mut task_context: TaskExecutionContext = TaskExecutionContext {
            task_objective  : instruction.into(),
            trial           : 1,
            previous_output : None,
            feedback        : None,
        };
        let mut final_output: Option<String> = None;

        while task_context.trial <= MAX_RETRIES {

            // task handling
            let worker_prompt     : String = task_context.to_worker_prompt();
            let worker_full_output: String = StreamForwarder::generate(
                worker,
                &worker_prompt,
                &client,
                board.clone(),
            ).await;

            task_context.previous_output = Some(worker_full_output.clone());

            if worker_full_output.is_empty() {
                return Err(format!(
                    "[DISPATCHER] worker on {} did not send any content.",
                    socket_path
                ));
            }

            println!("[DISPATCHER] worker output for task [{}] transferred to sentinel {}", task, sentinel_socket);
            
            // sentinel validation
            let sentinel_prompt     : String = task_context.to_sentinel_prompt(&worker_full_output);
            let sentinel_full_output: String = StreamForwarder::generate(
                "Sentinel",
                &sentinel_prompt,
                &sentinel_client,
                board.clone(),
            ).await;
            let sentinel_verdict    : SentinelVerdict = SentinelVerdict::from_json(&sentinel_full_output)
                .map_err(|e| format!("[DISPATCHER] failed to parse sentinel output as JSON: {}", e))?;

            task_context.feedback = Some(sentinel_verdict.clone());

            // Notify final output to Board
            let mut b: MutexGuard<'_, Board> = board.lock().await;
            b.publish(BoardMessage {
                agent    : "Runner".into(),
                content  : format!(
                    "Trial {}/{} for task [{}] completed with Sentinel verdict:
                    \n\n{}
                    \n\nGenerated response:\n{}",
                    task_context.trial,
                    MAX_RETRIES,
                    task,
                    sentinel_verdict.to_feedback(),
                    worker_full_output,
                ),
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;

            if sentinel_verdict.compliant {
                final_output = Some(worker_full_output);
                break;
            }

            task_context.trial += 1;

        }

        match final_output {
            Some(output) => Ok(output),
            None => Err(format!("[DISPATCHER] Sentinel rejected output after {} attempts", MAX_RETRIES)),
        }
    }
}
// src/engine/dispatcher.rs

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::board::{Board, BoardMessage};
use crate::core::mission::MissionNode;
use crate::config::SystemRegistry;

use crate::agents::agent_llm::AgentLLM;
use chrono::Utc;

use crate::utils::stream_forwarder::StreamForwarder;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentinelVerdict {
    pub compliant: bool,
    pub score    : u8,

    #[serde(rename = "missing_requirements")]
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
    pub fn to_coder_prompt(&self) -> String {
        let mut prompt: String =
            format!(
                "[TASK OBJECTIVE]\n\n{}\n",
                self.task_objective
            );

        if let Some(code) = &self.previous_output {
            prompt.push_str(
                &format!(
                    "\n[PREVIOUS GENERATED OUTPUT]\n\n{}\n",
                    code
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

const MAX_RETRIES: usize = 3;

pub struct AgentDispatcher {
    registry: Arc<SystemRegistry>,
}

impl AgentDispatcher {
    pub fn new(registry: Arc<SystemRegistry>) -> Self {
        Self { registry }
    }

    pub async fn execute(
        &self,
        node: &MissionNode,
        board: Arc<Mutex<Board>>,
    ) -> Result<String, String> {

        // specialized agent / worker
        let socket_path: &str = self.registry
            .resolve_worker(node.worker.as_deref().unwrap_or("coder"))
            .ok_or_else(|| format!("[DISPATCHER] unknown worker {:?}", node.worker))?;
        let client: AgentLLM = AgentLLM::new(socket_path);
        println!("[DISPATCHER] dispatching node [{}] to {}", node.id, socket_path);

        // sentinel
        let sentinel_socket: &str = self.registry
            .resolve_sentinel("sentinel-code")
            .ok_or_else(|| format!("[DISPATCHER] sentinel not found for worker {:?}", node.worker))?;
        let sentinel_client: AgentLLM = AgentLLM::new(sentinel_socket);

        // task description
        let instruction: String = node.description.clone().unwrap_or_else(|| node.title.clone());

        // initialize task execution context
        let mut task_context: TaskExecutionContext = TaskExecutionContext {
            task_objective  : instruction.clone(),
            trial           : 1,
            previous_output : None,
            feedback        : None,
        };
        let mut final_output: Option<String> = None;

        while task_context.trial <= MAX_RETRIES {
            // task handling
            let coder_prompt: String = task_context.to_coder_prompt();
            let coder_full_output: String = StreamForwarder::generate(
                "Coder",
                &coder_prompt,
                &client,
                board.clone(),
            ).await;

            task_context.previous_output = Some(coder_full_output.clone());

            if coder_full_output.is_empty() {
                return Err(format!(
                    "[DISPATCHER] specializedAgent on {} did not send any content.",
                    socket_path
                ));
            }

            println!("[DISPATCHER] code generated for node [{}]\n\tdispatching to sentinel {}", node.id, sentinel_socket);
            // sentinel validation
            let sentinel_prompt: String = task_context.to_sentinel_prompt(&coder_full_output);
            let sentinel_full_output: String = StreamForwarder::generate(
                "Sentinel",
                &sentinel_prompt,
                &sentinel_client,
                board.clone(),
            ).await;
            let sentinel_verdict: SentinelVerdict = SentinelVerdict::from_json(&sentinel_full_output)
                .map_err(|e| format!("[DISPATCHER] failed to parse sentinel output as JSON: {}", e))?;

            task_context.feedback = Some(sentinel_verdict.clone());

            // Notify final output to Board
            let mut b: tokio::sync::MutexGuard<'_, Board> = board.lock().await;
            b.publish(BoardMessage {
                agent    : "Runner".into(),
                content  : format!(
                    "Trial {}/{} for node [{}] completed with Sentinel verdict:
                    \n\n{}
                    \n\nGenerated response:\n{}",
                    task_context.trial,
                    MAX_RETRIES,
                    node.title,
                    sentinel_verdict.to_feedback(),
                    coder_full_output,
                ),
                timestamp: Utc::now().timestamp_millis() as u64,
            }).await;

            if sentinel_verdict.compliant {
                final_output = Some(coder_full_output);
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
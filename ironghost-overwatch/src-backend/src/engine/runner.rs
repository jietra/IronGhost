// src/engine/runner.rs

/// Parallel task scheduler from mission DiGraph

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::{mpsc, Mutex};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use crate::core::board::{Board, BoardMessage, MissionNode, BoardEvent};
use crate::engine::dag::MissionDag;

use crate::agents::agent_llm::AgentLLM;
use tokio::pin;                     // needed for .next()
use chrono::Utc;
use futures_util::StreamExt;        // needed for .next()

pub struct MissionRunner {
    board: Arc<Mutex<Board>>,
}

impl MissionRunner {
    pub fn new(board: Arc<Mutex<Board>>) -> Self {
        Self { board }
    }

    /// Listen to the Board in background and launch/replace DAG execution
    pub fn spawn_listener(self) -> JoinHandle<()> {
        let board = self.board.clone();

        tokio::spawn(async move {
            // 1. Retrieve broadcast receiver
            let mut rx = {
                let b = board.lock().await;
                b.tx.subscribe()
            };

            // Handle current running task (in case of cancellation through mission update)
            let mut current_task: Option<JoinHandle<()>> = None;
            println!("[RUNNER] started Board listener");

            // 2. board event listening loop
            while let Ok(event) = rx.recv().await {
                if let BoardEvent::MissionState(new_mission) = event {
                    println!("[RUNNER] new mission graph detected");

                    // 3. abort active execution on replanning
                    if let Some(handle) = current_task.take() {
                        println!("[RUNNER] cancelling previous DAG...");
                        handle.abort();
                    }

                    // 4. compiling Mission into MissionDag (petgraph)
                    match MissionDag::try_from_mission(&new_mission) {
                        Ok(dag) => {
                            let board_clone = board.clone();
                            
                            // 5. spawn new execution
                            current_task = Some(tokio::spawn(async move {
                                let runner = MissionRunner::new(board_clone);
                                runner.run_mission(dag).await;
                            }));
                        }
                        Err(err) => {
                            eprintln!("[RUNNER] DAG compilation error: {}", err);
                        }
                    }
                }
            }
        })
    }

    pub async fn run_mission(&self, dag: MissionDag) {
        let total_nodes = dag.graph.node_count();
        if total_nodes == 0 { return; }

        // compute nb of dependencies (in-degree) for each node
        let mut in_degrees: Vec<usize> = dag.graph
            .node_indices()
            .map(|idx| dag.graph.edges_directed(idx, petgraph::Direction::Incoming).count())
            .collect();

        // open an MPSC channel to listen to end of executions for parallel tasks
        let (tx, mut rx) = mpsc::channel::<(NodeIndex, Result<String, String>)>(32);

        // 1. trigger initial root nodes (in_degree == 0)
        for idx in dag.graph.node_indices() {
            if in_degrees[idx.index()] == 0 {
                Self::spawn_task(&dag.graph[idx], idx, tx.clone(), Arc::clone(&self.board));
            }
        }

        // 2. event-driven resolution loop
        let mut completed_count = 0;

        while let Some((completed_idx, _result)) = rx.recv().await {
            completed_count += 1;

            // unlock childs of completed node
            for edge in dag.graph.edges(completed_idx) {
                let child_idx = edge.target();
                let child_deg = &mut in_degrees[child_idx.index()];
                *child_deg -= 1;

                // if all child dependencies are lifted -> Spawn parallel
                if *child_deg == 0 {
                    Self::spawn_task(&dag.graph[child_idx], child_idx, tx.clone(), Arc::clone(&self.board));
                }
            }

            if completed_count == total_nodes {
                println!("[RUNNER] === Mission accomplished! ===");
                
                // Notify board
                let mut b = self.board.lock().await;
                b.publish(BoardMessage {
                    agent    : "Runner".into(),
                    content  : "Mission completed successfully".into(),
                    timestamp: 0,
                }).await;
                break;
            }
        }
    }

    fn spawn_task(
        node    : &MissionNode,
        node_idx: NodeIndex,
        tx      : mpsc::Sender<(NodeIndex, Result<String, String>)>,
        board   : Arc<Mutex<Board>>,
    ) {
        let node = node.clone();
        
        tokio::spawn(async move {
            println!("[RUNNER] Launching task: [{}] {}", node.kind, node.title);
            
            // executing logic according to node type
            let result = match node.kind.as_str() {
                "task" | "subgoal" => {
                    
                    // -----------------------------
                    // call Worker, Lead or Sentinel
                    // -----------------------------
                    //tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    //Ok(format!("Task '{}' successfully done", node.title))

                    // 1. fetch assigned specialized agent and instruction
                    let socket_path = resolve_worker_socket(&node);
                    let prompt = node.description.clone().unwrap_or_else(|| node.title.clone());

                    println!("[RUNNER] Dispatching node [{}] to {}", node.id, socket_path);

                    // 2. call specialized agent to handle task
                    let client = AgentLLM::new(socket_path);

                    let msg_id = format!("{}-{}", "Runner", Utc::now().timestamp_millis());
                    let tx_board = board.lock().await.tx.clone();

                    // start of stream -> Notif UI via Board
                    let _ = tx_board.send(BoardEvent::StreamStart {
                        msg_id: msg_id.clone(),
                        agent : "Runner".into(),
                    });

                    // process stream
                    let mut full_output = String::new();
                    let stream = client.stream_generate(&prompt);
                    pin!(stream);   // pin stream on stack to authorize .next()
                    
                    while let Some(chunk) = stream.next().await {
                        full_output.push_str(&chunk);

                        // send each chunk on broadcast channel for UI
                        let _ = tx_board.send(BoardEvent::StreamChunk {
                            msg_id: msg_id.clone(),
                            delta : chunk,
                        });
                    }

                    // send end of stream
                    let _ = tx_board.send(BoardEvent::StreamEnd {
                        msg_id: msg_id.clone(),
                    });

                    if full_output.is_empty() {
                        Err(format!("SpecializedAgent on {} did not send any content.", socket_path))
                    } else {
                        println!("[RUNNER] Final code generated for node [{}]", node.id);

                        // Notify final output to Board
                        let mut b = board.lock().await;
                        b.publish(BoardMessage {
                            agent    : "Runner".into(),
                            content  : format!("Task [{}] completed with result:\n{}\n", node.id, full_output),
                            timestamp: Utc::now().timestamp_millis() as u64,
                        }).await;

                        Ok(full_output)
                    }
                }
                _ => Ok("Asset/Goal ignored by runner execution engine".into()), // "assets" or "risks" are only informative at this stage
            };
            
            // Inform scheduler to unlock child task
            let _ = tx.send((node_idx, result)).await;
        });
    }

}

// TODO: use a proper HashMap instead of using match
fn resolve_worker_socket(node: &MissionNode) -> &'static str {
    match node.worker.as_deref() {
        Some("coder") => "/tmp/agent_coder.sock",
        //Some("sentinel") => "/tmp/agent_sentinel.sock",
        //Some("executor") => "/tmp/agent_executor.sock",
        // Fallback
        _ => match node.kind.as_str() {
            "task" => "/tmp/agent_coder.sock",
            _ => "/tmp/agent_coder.sock",
        },
    }
}
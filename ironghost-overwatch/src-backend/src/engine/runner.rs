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

                    /*
                    match Self::call_worker_uds(socket_path, &prompt).await {
                        Ok(generated_code) => {
                            println!("[RUNNER] Output received for node [{}]", node.id);

                            // 3. Notify result on Board
                            let mut b = board.lock().await;
                            b.publish(BoardMessage {
                                agent    : "Runner".into(),
                                content  : format!("Task [{}] completed:\n```\n{}\n```", node.id, generated_code),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            }).await;

                            Ok(generated_code)
                        }
                        Err(err) => Err(format!("Worker failed on node [{}]: {}", node.id, err)),
                    }
                    */
                }
                _ => Ok("Asset/Goal ignored by runner execution engine".into()), // "assets" or "risks" are only informative at this stage
            };
            
            //let result = Self::call_worker_uds(socket_path, &prompt).await;

            // Notify board
            //let mut b = board.lock().await;
            //b.publish(BoardMessage {
            //    agent    : "Runner".into(),
            //    content  : format!("Node [{}] finished: {:?}", node.id, result),
            //    timestamp: 0,
            //}).await;

            // Inform scheduler to unlock child task
            let _ = tx.send((node_idx, result)).await;
        });
    }

    /*
    /// Connect to specialized agent / worker via Unix Domain Socket
    async fn call_worker_uds(socket_path: &str, payload: &str) -> Result<String, String> {
        let mut stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| format!("UDS Connection error to {}: {}", socket_path, e))?;

        // send payload to socket
        let data   = payload.as_bytes();
        let len_bytes = (data.len() as u32).to_le_bytes();

        stream.write_all(&len_bytes)
            .await
            .map_err(|e| format!("UDS Write error (len): {}", e))?;
        stream.write_all(data)
            .await
            .map_err(|e| format!("UDS Write error (payload): {}", e))?;

        // close write stream, signaling EOF
        stream.shutdown().await.map_err(|e| e.to_string())?;

        // read stream
        let mut utf8_buffer  = Vec::new();
        let mut response = String::new();

        loop {
            // read length of token
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                println!("[AGENTLLM] stream len not exact! break!");
                break;
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // read token
            let mut token_buf = vec![0u8; len];
            //stream.read_exact(&mut token_buf).await.unwrap();
            if stream.read_exact(&mut token_buf).await.is_err() {
                return Err("UDS payload stream truncated unexpectedly".into());
            }

            // end of stream
            if token_buf == b"\n=== END_OF_STREAM ===" {
                println!("\n\n[AGENTLLM] end of stream! break!");
                break;
            }

            // accumulate UTF-8 fragments
            utf8_buffer.extend_from_slice(&token_buf);

            if let Ok(s) = std::str::from_utf8(&utf8_buffer) {
                response.push_str(s);
                print!("{}", s);
                utf8_buffer.clear();
            }
        }

        Ok(response)
    }
    */

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
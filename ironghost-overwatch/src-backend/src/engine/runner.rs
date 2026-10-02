// src/engine/runner.rs

/// Parallel task scheduler from mission DiGraph

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::{mpsc, Mutex};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use crate::core::board::{Board, BoardMessage, MissionNode, BoardEvent};
use crate::engine::dag::MissionDag;

use crate::engine::registry::AgentRegistry;
use crate::engine::dispatcher::AgentDispatcher;

pub struct MissionRunner {
    board   : Arc<Mutex<Board>>,
    registry: Arc<AgentRegistry>,
}

impl MissionRunner {
    pub fn new(board: Arc<Mutex<Board>>, registry: Arc<AgentRegistry>) -> Self {
        Self {
            board,
            registry,
        }
    }

    /// Listen to the Board in background and launch/replace DAG execution
    pub fn spawn_listener(self) -> JoinHandle<()> {
        let board   : Arc<Mutex<Board>>  = self.board.clone();
        let registry: Arc<AgentRegistry> = self.registry.clone();
        
        tokio::spawn(async move {
            // 1. Retrieve broadcast receiver
            let mut rx: tokio::sync::broadcast::Receiver<BoardEvent> = {
                let b: tokio::sync::MutexGuard<'_, Board> = board.lock().await;
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
                            let board_clone: Arc<Mutex<Board>> = board.clone();
                            
                            // 5. spawn new execution
                            let registry_for_mission: Arc<AgentRegistry> = registry.clone();
                            current_task = Some(tokio::spawn(async move {
                                //let runner = MissionRunner::new(board_clone);
                                let runner = MissionRunner::new(
                                    board_clone,
                                    registry_for_mission,
                                );
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
                Self::spawn_task(&dag.graph[idx], idx, tx.clone(), Arc::clone(&self.board), Arc::clone(&self.registry));
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
                    Self::spawn_task(&dag.graph[child_idx], child_idx, tx.clone(), Arc::clone(&self.board), Arc::clone(&self.registry));
                }
            }

            if completed_count == total_nodes {
                println!("[RUNNER] === Mission accomplished! ===");
                
                // Notify board
                let mut b: tokio::sync::MutexGuard<'_, Board> = self.board.lock().await;
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
        registry: Arc<AgentRegistry>
    ) {
        let node: MissionNode = node.clone();
        
        tokio::spawn(async move {
            println!("[RUNNER] Launching task: [{}] {}", node.kind, node.title);
            
            // executing logic according to node type
            let result: Result<String, String> = match node.kind.as_str() {
                "task" | "subgoal" => {
                    let dispatcher: AgentDispatcher = AgentDispatcher::new(registry);
                    dispatcher.execute(&node, board.clone()).await
                }
                _ => Ok("Asset/Goal ignored by runner execution engine".into()), // "assets" or "risks" are only informative at this stage
            };
            
            // Inform scheduler to unlock child task
            let _ = tx.send((node_idx, result)).await;
        });
    }

}
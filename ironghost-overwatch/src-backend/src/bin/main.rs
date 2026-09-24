use backend::{
    core::board::{Board, BoardMessage},
    agents::agent::Agent,
    agents::agent_llm::AgentLLM,
};
use std::sync::Arc;         // share an object (Board) between multiple tasks
use tokio::sync::Mutex;     // mutex async (block tasks, not threads)

#[tokio::main]  // macro-attribute launching Tokio runtime with main() as unique entry point (for async: futures...)
async fn main() {
    // 1. init board
    let board = Arc::new(Mutex::new(Board::new()));
    //let board = Board::new();

    // 2. create broadcast channel
    // cannot use board.tx directly since board is a Arc<tokio::Mutex<Board>>.
    let tx = {
        // unlock mutex first to access actual board (board is a shared pointer to a mutex)
        // this block {...} allows a direct freeing of the mutex (the gard is immediately destroyed).
        // otherwise, the wait would last till end of scope
        let board_guard = board.lock().await;
        board_guard.tx.clone()
    };

    // 3. create agents
    let agents = vec![
        Agent::new(
            "Strategist",
            board.clone(),
            tx.subscribe(),                 // specific rx
            AgentLLM::new("/tmp/agent_strategist.sock")  // specific LLM
        ),
        Agent::new(
            "Coder",
            board.clone(),
            tx.subscribe(),
            AgentLLM::new("/tmp/agent_coder.sock")
        ),
        Agent::new(
            "Generic",
            board.clone(),
            tx.subscribe(),
            AgentLLM::new("/tmp/agent_generic.sock")
        ),
    ];

    // 4. run agents (via spawn)
    for agent in agents {
        agent.spawn();
    }
    println!("[MAIN] agents spawned!");

    tokio::spawn(websocket_ui_server(board.clone()));

    println!("[MAIN] Waiting for agents...");
    tokio::signal::ctrl_c().await.unwrap();
    println!("[MAIN] Shutting down...");

}


async fn websocket_ui_server(board: Arc<Mutex<Board>>) {
    use futures_util::{StreamExt, SinkExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::Message;

    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("[WS-UI] UI WebSocket server running on ws://127.0.0.1:9000");

    while let Ok((stream, _)) = listener.accept().await {
        let ws = accept_async(stream).await.unwrap();
        let (ws_tx_raw, mut ws_rx) = ws.split();

        // clone sink for each task
        let ws_tx = Arc::new(Mutex::new(ws_tx_raw));

        // Broadcast the board -> UI
        {
            let board = board.clone();
            let ws_tx_board   = ws_tx.clone();

            tokio::spawn(async move {
                let mut rx = board.lock().await.tx.subscribe();

                while let Ok(event) = rx.recv().await {
                    //let json = serde_json::to_string(&msg).unwrap();
                    //let _ = ws_tx_board.lock().await.send(Message::Text(json)).await;
                    /*let msg = match event {
                        BoardEvent::NewMessage(msg) => msg,
                        BoardEvent::MissionState(_) => {
                            // ignored for now
                            continue;
                        }
                    };*/
                    if let Ok(json) = serde_json::to_string(&event) {
                        let _ = ws_tx_board.lock().await.send(Message::Text(json)).await;
                    }
                }
            });
        }

        // Receiving requests/messages (UI -> board)
        {
            let board = board.clone();
            let ws_tx_ui = ws_tx.clone();

            tokio::spawn(async move {
                while let Some(Ok(Message::Text(text))) = ws_rx.next().await {

                    // try to parse a generic message with field "type"
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {

                        // if get_history
                        if json.get("type") == Some(&serde_json::Value::String("get_history".into())) {
                            // retrieve board history
                            println!("[WS-UI] retrieve board history");
                            //let history = board.lock().await.messages.clone();
                            let board_guard = board.lock().await;


                            // send history to client
                            let payload = serde_json::json!({
                                "type": "history",
                                //"payload": history
                                "payload": board_guard.messages
                            });

                            println!("[WS-UI] send board history to UI");
                            let _ = ws_tx_ui.lock().await.send(Message::Text(payload.to_string())).await;

                            // send initial state of mission graph
                            let mission_payload = serde_json::json!({
                                "type": "mission_state",
                                "payload": board_guard.mission
                            });
                            let _ = ws_tx_ui.lock().await.send(Message::Text(mission_payload.to_string())).await;

                            continue;
                        }
                    }

                    // otherwise, try to parse normal BoardMessage
                    if let Ok(msg) = serde_json::from_str::<BoardMessage>(&text) {
                        println!("[WS-UI] publish message to board");
                        board.lock().await.publish(msg).await;
                    }
                }
            });
        }
    }
}

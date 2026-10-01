// src/ui/websocket.rs

use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};

use crate::core::board::{Board, BoardMessage};

pub async fn run_websocket_ui(addr: &str, board: Arc<Mutex<Board>>) {
    let listener = TcpListener::bind(addr).await.expect("Impossible to bind port WS");
    println!("[WS-UI] UI WebSocket server running on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let board_clone = board.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, board_clone).await {
                eprintln!("[WS-UI] connexion error : {:?}", e);
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream, 
    board : Arc<Mutex<Board>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws = accept_async(stream).await?;
    let (ws_tx_raw, mut ws_rx) = ws.split();

    // clone sink for each task
    let ws_tx = Arc::new(Mutex::new(ws_tx_raw));

    // Task 1: Board -> UI (Broadcast)
    {
        let board = board.clone();
        let ws_tx = ws_tx.clone();
        tokio::spawn(async move {
            let mut rx = board.lock().await.tx.subscribe();
            while let Ok(event) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&event) {
                    let _ = ws_tx.lock().await.send(Message::Text(json)).await;
                }
            }
        });
    }

    // Task 2: UI -> Board
    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {

        // try to parse a generic message with field "type"
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {

            // if get_history
            if json.get("type") == Some(&serde_json::Value::String("get_history".into())) {
                println!("[WS-UI] retrieving board history");
                let board_guard = board.lock().await;

                // send history to client
                let payload = serde_json::json!({
                    "type": "history",
                    "payload": board_guard.messages
                });
                println!("[WS-UI] send board history to UI");
                let _ = ws_tx.lock().await.send(Message::Text(payload.to_string())).await;

                // send initial state of mission graph
                let mission_payload = serde_json::json!({
                    "type": "mission_state",
                    "payload": board_guard.mission
                });
                let _ = ws_tx.lock().await.send(Message::Text(mission_payload.to_string())).await;

                continue;
            }
        }

        // otherwise, try to parse normal BoardMessage
        if let Ok(msg) = serde_json::from_str::<BoardMessage>(&text) {
            println!("[WS-UI] publish message to board");
            board.lock().await.publish(msg).await;
        }
    }

    Ok(())
}
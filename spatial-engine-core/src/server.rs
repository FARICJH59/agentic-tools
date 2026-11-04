use axum::{
    routing::get,
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    extract::State,
    response::IntoResponse,
    Json, Router,
};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use crate::crdt::{SpatialRegistry, SpatialEntityState};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<Mutex<SpatialRegistry>>,
    pub tx: broadcast::Sender<SpatialEntityState>,
}

pub type SharedState = Arc<AppState>;

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/ws-4d-sync", get(ws_handler))
        .route("/state", get(get_state_handler))
        .with_state(state)
}

async fn get_state_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let reg = state.registry.lock().await;
    Json(reg.clone())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to the broadcast channel for other nodes' deltas
    let mut rx = state.tx.subscribe();

    // Spawn a task to forward broadcasted deltas to this specific websocket client
    let mut send_task = tokio::spawn(async move {
        while let Ok(state_delta) = rx.recv().await {
            if let Ok(json_str) = serde_json::to_string(&state_delta) {
                if sender.send(Message::Text(json_str)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn a task to listen for incoming deltas from this client
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            if let Ok(msg) = result {
                if let Message::Text(text) = msg {
                    if let Ok(state_delta) = serde_json::from_str::<SpatialEntityState>(&text) {
                        // 1. Apply to CRDT Registry
                        let mut reg = state_clone.registry.lock().await;
                        if let Err(e) = reg.apply_delta(state_delta.clone()) {
                            eprintln!("[HOARE Runtime Violation Rejected] {}", e);
                            continue;
                        }
                        println!("[Spatial-4D Core] Applied delta from [{}]. Active entities: {}", state_delta.node_id, reg.entities.len());
                        drop(reg);

                        // 2. Broadcast to all connected peers
                        let _ = state_clone.tx.send(state_delta);
                    } else {
                        println!("[Spatial-4D Core] Failed to parse JSON delta: {}", text);
                    }
                }
            } else {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

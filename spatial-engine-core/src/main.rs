use std::sync::Arc;
use tokio::sync::broadcast;
use spatial_engine_core::crdt::SpatialRegistry;
use spatial_engine_core::server::{create_router, AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let registry = Arc::new(tokio::sync::Mutex::new(SpatialRegistry::new()));
    let (tx, _rx) = broadcast::channel(100);

    let state = Arc::new(AppState { registry, tx });
    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[Spatial-4D Engine Core] Running multi-user server on http://{}", addr);
    println!("[Spatial-4D Engine Core] Multi-peer WebSocket ready at ws://{}/ws-4d-sync", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

mod config;
mod error;
mod state;
mod store;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use store::memory::InMemoryStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();

    let config = config::Config::from_env();
    let store = InMemoryStore::default();
    let state = AppState::new(config.clone(), store);

    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", config.http_port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let room_count = state.store.list_rooms().await.len();
    Json(json!({ "status": "ok", "rooms": room_count }))
}

pub mod auth;
pub mod config;
pub mod error;
pub mod rooms;
pub mod signaling;
pub mod sfu;
pub mod state;
pub mod store;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use store::memory::InMemoryStore;
use tower_http::cors::CorsLayer;

pub async fn create_app() -> Router {
    let config = config::Config::from_env();
    let store = InMemoryStore::default();
    let state = AppState::new(config, store).await.expect("failed to initialize SFU state");
    build_router(state)
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .nest("/auth", auth::router())
        .nest("/rooms", rooms::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let room_count = state.store.list_rooms().await.len();
    Json(json!({ "status": "ok", "rooms": room_count }))
}

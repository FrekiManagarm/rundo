pub mod auth;
pub mod config;
pub mod error;
pub mod state;
pub mod store;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use store::memory::InMemoryStore;
use tower_http::cors::CorsLayer;

pub fn create_app() -> Router {
    let config = config::Config::from_env();
    let store = InMemoryStore::default();
    let state = AppState::new(config, store);
    Router::new()
        .route("/health", get(health_handler))
        .nest("/auth", auth::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let room_count = state.store.list_rooms().await.len();
    Json(json!({ "status": "ok", "rooms": room_count }))
}

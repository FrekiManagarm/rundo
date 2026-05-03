pub mod auth;
pub mod config;
pub mod error;
pub mod rooms;
pub mod signaling;
pub mod state;
pub mod store;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use tower_http::cors::CorsLayer;

pub async fn create_app() -> Router {
    let config = config::Config::from_env();
    let state = build_state(config).await.expect("failed to initialize app state");
    build_router(state)
}

async fn build_state(config: config::Config) -> anyhow::Result<AppState> {
    if config.database_url.starts_with("postgres") {
        let store = store::postgres::PostgresStore::new(&config.database_url).await?;
        AppState::new(config, store).await
    } else {
        let store = store::sqlite::SqliteStore::new(&config.database_url).await?;
        AppState::new(config, store).await
    }
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

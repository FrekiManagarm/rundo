pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod password;

use axum::{routing::post, Router};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/token", post(handlers::login))
}

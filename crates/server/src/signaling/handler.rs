use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;

use crate::state::AppState;

pub async fn ws_handler(
    _ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> Response {
    todo!("implemented in Task 13")
}

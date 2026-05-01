use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::Response,
};
use shared::models::RoomId;

use crate::{auth::middleware::AuthUser, error::AppError, state::AppState};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<RoomId>,
    AuthUser(user_id): AuthUser,
) -> Result<Response, AppError> {
    state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;

    let cmd_tx = state
        .registry
        .get_cmd_tx(room_id)
        .ok_or_else(|| AppError::NotFound("room not active".to_string()))?;

    let sfu = state.sfu.clone();

    Ok(ws.on_upgrade(move |socket| {
        crate::signaling::session::run_session(socket, user_id, room_id, cmd_tx, sfu)
    }))
}

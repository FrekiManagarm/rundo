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
    let room = state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;

    // Lazily start the room task if it dropped from the registry (e.g. after server restart).
    let cmd_tx = match state.registry.get_cmd_tx(room_id) {
        Some(tx) => tx,
        None => {
            let (tx, rx) = tokio::sync::mpsc::channel(256);
            state.registry.insert(room, tx.clone());
            tokio::spawn(crate::rooms::room::run_room(room_id, rx));
            tx
        }
    };

    let sfu = state.sfu.clone();

    Ok(ws.on_upgrade(move |socket| {
        crate::signaling::session::run_session(socket, user_id, room_id, cmd_tx, sfu)
    }))
}

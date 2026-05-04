use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::models::{Room, RoomId, RoomKind, UserId};

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub kind: RoomKind,
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub id: RoomId,
    pub name: String,
    pub kind: RoomKind,
    pub owner_id: UserId,
    pub peer_count: usize,
}

impl RoomResponse {
    fn from_room(room: Room, peer_count: usize) -> Self {
        Self {
            id: room.id,
            name: room.name,
            kind: room.kind,
            owner_id: room.owner_id,
            peer_count,
        }
    }
}

pub async fn create_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreateRoomRequest>,
) -> AppResult<(StatusCode, Json<RoomResponse>)> {
    let room = Room {
        id: RoomId::new(),
        name: body.name,
        kind: body.kind,
        owner_id: user_id,
        created_at: Utc::now(),
    };
    state.store.create_room(room.clone()).await.map_err(|e| {
        tracing::error!("create_room db error: {e}");
        AppError::Internal(e)
    })?;

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(256);
    let peer_counter = state.registry.insert(room.clone(), cmd_tx);
    let server_addr = state.config.server_addr.clone();
    tokio::spawn(crate::rooms::room::run_room(room.id, cmd_rx, peer_counter, server_addr));

    Ok((StatusCode::CREATED, Json(RoomResponse::from_room(room, 0))))
}

pub async fn list_rooms(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
) -> AppResult<Json<Vec<RoomResponse>>> {
    let rooms = state.store.list_rooms().await;
    let responses = rooms
        .into_iter()
        .map(|r| {
            let count = state.registry.peer_count(r.id);
            RoomResponse::from_room(r, count)
        })
        .collect();
    Ok(Json(responses))
}

pub async fn get_room(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(room_id): Path<RoomId>,
) -> AppResult<Json<RoomResponse>> {
    let room = state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;
    let count = state.registry.peer_count(room_id);
    Ok(Json(RoomResponse::from_room(room, count)))
}

pub async fn delete_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<RoomId>,
) -> AppResult<StatusCode> {
    let room = state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;
    if room.owner_id != user_id {
        return Err(AppError::Forbidden);
    }
    state.store.delete_room(room_id).await.map_err(AppError::Internal)?;
    state.registry.remove(room_id);
    Ok(StatusCode::NO_CONTENT)
}

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{
    messages::{ClientMessage, ServerMessage},
    models::{PeerId, PeerInfo, RoomId, UserId},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::rooms::registry::RoomCommand;

pub async fn run_session(
    socket: WebSocket,
    user_id: UserId,
    _room_id: RoomId,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
) {
    let peer_id = PeerId::new();
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (server_msg_tx, mut server_msg_rx) = mpsc::channel::<ServerMessage>(256);

    let peer_info = PeerInfo { peer_id, user_id, connected_at: Utc::now() };
    let _ = room_cmd_tx
        .send(RoomCommand::PeerJoined { peer_id, info: peer_info, ws_tx: server_msg_tx })
        .await;

    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Answer { sdp }) => {
                                let _ = room_cmd_tx.send(RoomCommand::PeerAnswer { peer_id, sdp }).await;
                            }
                            Ok(ClientMessage::IceCandidate { candidate }) => {
                                let _ = room_cmd_tx
                                    .send(RoomCommand::PeerIceCandidate { peer_id, candidate })
                                    .await;
                            }
                            Ok(ClientMessage::ChatMessage { text }) => {
                                let timestamp_ms = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as i64;
                                let _ = room_cmd_tx
                                    .send(RoomCommand::BroadcastChat { from_peer: peer_id, text, timestamp_ms })
                                    .await;
                            }
                            Err(e) => tracing::debug!("peer {peer_id:?} parse error: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            Some(server_msg) = server_msg_rx.recv() => {
                let json = match serde_json::to_string(&server_msg) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                if ws_tx.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = room_cmd_tx.send(RoomCommand::PeerLeft { peer_id }).await;
}

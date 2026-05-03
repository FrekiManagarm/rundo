use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{
    messages::{ClientMessage, ServerMessage},
    models::{PeerId, PeerInfo, RoomId, UserId},
};
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

    if send_msg(&mut ws_tx, &ServerMessage::Joined { peer_id }).await.is_err() {
        return;
    }

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
                            Ok(ClientMessage::Leave) => break,
                            Ok(ClientMessage::OfferTo { target, sdp }) => {
                                let _ = room_cmd_tx.send(RoomCommand::Relay {
                                    to: target,
                                    msg: ServerMessage::OfferFrom { from_peer: peer_id, sdp },
                                }).await;
                            }
                            Ok(ClientMessage::AnswerTo { target, sdp }) => {
                                let _ = room_cmd_tx.send(RoomCommand::Relay {
                                    to: target,
                                    msg: ServerMessage::AnswerFrom { from_peer: peer_id, sdp },
                                }).await;
                            }
                            Ok(ClientMessage::IceCandidateTo { target, candidate }) => {
                                let _ = room_cmd_tx.send(RoomCommand::Relay {
                                    to: target,
                                    msg: ServerMessage::IceCandidateFrom {
                                        from_peer: peer_id,
                                        candidate,
                                    },
                                }).await;
                            }
                            Err(e) => tracing::debug!("peer {peer_id:?} parse error: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            Some(server_msg) = server_msg_rx.recv() => {
                if send_msg(&mut ws_tx, &server_msg).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = room_cmd_tx.send(RoomCommand::PeerLeft { peer_id }).await;
}

async fn send_msg(
    tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(msg)?;
    tx.send(Message::Text(json.into())).await?;
    Ok(())
}

use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{
    messages::{ClientMessage, ServerMessage},
    models::{PeerId, PeerInfo, RoomId, UserId},
};
use tokio::sync::{mpsc, oneshot};

use crate::{
    rooms::{
        peer::PeerTask,
        registry::{RoomCommand, RtpPayload},
    },
    sfu::{udp::DemuxControl, SfuState},
};

pub async fn run_session(
    socket: WebSocket,
    user_id: UserId,
    _room_id: RoomId,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
    sfu: SfuState,
) {
    let peer_id = PeerId::new();
    let (mut ws_tx, mut ws_rx) = socket.split();

    let joined = ServerMessage::Joined { peer_id };
    if send_msg(&mut ws_tx, &joined).await.is_err() {
        return;
    }

    let offer_sdp = loop {
        match ws_rx.next().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Offer { sdp }) => break sdp,
                Ok(ClientMessage::Leave) => return,
                _ => continue,
            },
            Some(Ok(Message::Close(_))) | None => return,
            _ => continue,
        }
    };

    let remote_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let (udp_tx, udp_rx) = mpsc::channel::<(SocketAddr, Vec<u8>)>(256);
    let (rtp_tx, rtp_rx) = mpsc::channel::<RtpPayload>(256);
    let (answer_tx, answer_rx) = oneshot::channel::<anyhow::Result<String>>();

    if remote_addr.port() != 0 {
        let _ = sfu
            .demux_ctrl
            .send(DemuxControl::Register { addr: remote_addr, tx: udp_tx })
            .await;
    }

    let peer_info = PeerInfo { peer_id, user_id, connected_at: Utc::now() };
    let _ = room_cmd_tx
        .send(RoomCommand::PeerJoined { peer_id, info: peer_info, rtp_tx })
        .await;

    tokio::spawn(crate::rooms::peer::run_peer(PeerTask {
        peer_id,
        socket: sfu.socket.clone(),
        local_addr: sfu.local_addr,
        remote_addr,
        sdp_offer: offer_sdp,
        room_cmd_tx: room_cmd_tx.clone(),
        demux_ctrl: sfu.demux_ctrl.clone(),
        udp_rx,
        rtp_rx,
        answer_tx,
    }));

    match answer_rx.await {
        Ok(Ok(sdp)) => {
            if send_msg(&mut ws_tx, &ServerMessage::Answer { sdp }).await.is_err() {
                return;
            }
        }
        _ => return,
    }

    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Leave) => break,
                Ok(ClientMessage::IceCandidate { candidate }) => {
                    tracing::debug!("peer {peer_id:?} ICE candidate: {candidate}");
                }
                _ => {}
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

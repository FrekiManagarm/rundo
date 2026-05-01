use std::collections::HashMap;

use shared::models::{PeerId, RoomId};
use tokio::sync::mpsc;

use crate::rooms::registry::{RoomCommand, RtpPayload};

struct PeerHandle {
    rtp_tx: mpsc::Sender<RtpPayload>,
}

pub async fn run_room(room_id: RoomId, mut cmd_rx: mpsc::Receiver<RoomCommand>) {
    let mut peers: HashMap<PeerId, PeerHandle> = HashMap::new();

    tracing::info!("room {room_id:?} started");

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            RoomCommand::PeerJoined { peer_id, info: _, rtp_tx } => {
                tracing::info!("room {room_id:?}: peer {peer_id:?} joined");
                peers.insert(peer_id, PeerHandle { rtp_tx });
            }
            RoomCommand::PeerLeft { peer_id } => {
                tracing::info!("room {room_id:?}: peer {peer_id:?} left");
                peers.remove(&peer_id);
                if peers.is_empty() {
                    tracing::info!("room {room_id:?} empty, shutting down");
                    break;
                }
            }
            RoomCommand::MediaData { from, payload } => {
                for (peer_id, handle) in &peers {
                    if *peer_id != from {
                        let _ = handle.rtp_tx.try_send(payload.clone());
                    }
                }
            }
        }
    }

    tracing::info!("room {room_id:?} stopped");
}

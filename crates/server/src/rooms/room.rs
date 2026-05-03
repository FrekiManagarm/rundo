use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use shared::{messages::ServerMessage, models::{PeerId, RoomId}};
use tokio::sync::mpsc;

use crate::rooms::registry::RoomCommand;

pub async fn run_room(
    room_id: RoomId,
    mut cmd_rx: mpsc::Receiver<RoomCommand>,
    peer_counter: Arc<AtomicUsize>,
) {
    let mut peers: HashMap<PeerId, mpsc::Sender<ServerMessage>> = HashMap::new();

    tracing::info!("room {room_id:?} started");

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            RoomCommand::PeerJoined { peer_id, info: _, ws_tx } => {
                tracing::info!("room {room_id:?}: peer {peer_id:?} joined ({} existing)", peers.len());
                // Notify the new peer of everyone already in the room
                for &existing_id in peers.keys() {
                    let _ = ws_tx.try_send(ServerMessage::ExistingPeer { peer_id: existing_id });
                }
                // Notify everyone else that a new peer joined
                for tx in peers.values() {
                    let _ = tx.try_send(ServerMessage::PeerJoined { peer_id });
                }
                peers.insert(peer_id, ws_tx);
                peer_counter.store(peers.len(), Ordering::Relaxed);
            }
            RoomCommand::PeerLeft { peer_id } => {
                tracing::info!("room {room_id:?}: peer {peer_id:?} left");
                peers.remove(&peer_id);
                peer_counter.store(peers.len(), Ordering::Relaxed);
                for tx in peers.values() {
                    let _ = tx.try_send(ServerMessage::PeerLeft { peer_id });
                }
                if peers.is_empty() {
                    tracing::info!("room {room_id:?} empty, shutting down");
                    break;
                }
            }
            RoomCommand::Relay { to, msg } => {
                if let Some(tx) = peers.get(&to) {
                    let _ = tx.try_send(msg);
                }
            }
        }
    }

    tracing::info!("room {room_id:?} stopped");
}

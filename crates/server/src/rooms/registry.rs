use dashmap::DashMap;
use shared::{
    messages::ServerMessage,
    models::{PeerInfo, Room, RoomId},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::mpsc;

use shared::models::PeerId;

#[derive(Debug)]
pub enum RoomCommand {
    PeerJoined {
        peer_id: PeerId,
        info: PeerInfo,
        ws_tx: mpsc::Sender<ServerMessage>,
    },
    PeerLeft {
        peer_id: PeerId,
    },
    Relay {
        to: PeerId,
        msg: ServerMessage,
    },
}

pub struct RoomHandle {
    pub room: Room,
    pub peer_count: Arc<AtomicUsize>,
    pub cmd_tx: mpsc::Sender<RoomCommand>,
}

#[derive(Default)]
pub struct RoomRegistry {
    rooms: DashMap<RoomId, RoomHandle>,
}

impl RoomRegistry {
    pub fn insert(&self, room: Room, cmd_tx: mpsc::Sender<RoomCommand>) -> Arc<AtomicUsize> {
        let peer_count = Arc::new(AtomicUsize::new(0));
        self.rooms.insert(
            room.id,
            RoomHandle { room, peer_count: peer_count.clone(), cmd_tx },
        );
        peer_count
    }

    pub fn get_room_meta(&self, id: RoomId) -> Option<Room> {
        self.rooms.get(&id).map(|h| h.room.clone())
    }

    pub fn list_rooms(&self) -> Vec<Room> {
        self.rooms.iter().map(|h| h.room.clone()).collect()
    }

    pub fn remove(&self, id: RoomId) {
        self.rooms.remove(&id);
    }

    pub fn get_cmd_tx(&self, id: RoomId) -> Option<mpsc::Sender<RoomCommand>> {
        self.rooms.get(&id).map(|h| h.cmd_tx.clone())
    }

    pub fn peer_count(&self, id: RoomId) -> usize {
        self.rooms.get(&id).map(|h| h.peer_count.load(Ordering::Relaxed)).unwrap_or(0)
    }
}

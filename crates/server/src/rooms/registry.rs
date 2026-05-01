use dashmap::DashMap;
use shared::models::{PeerId, PeerInfo, Room, RoomId};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum RoomCommand {
    PeerJoined {
        peer_id: PeerId,
        info: PeerInfo,
        rtp_tx: mpsc::Sender<RtpPayload>,
    },
    PeerLeft {
        peer_id: PeerId,
    },
    MediaData {
        from: PeerId,
        payload: RtpPayload,
    },
}

#[derive(Debug, Clone)]
pub struct RtpPayload {
    pub data: Vec<u8>,
    pub timestamp: u32,
    pub payload_type: u8,
}

pub struct RoomHandle {
    pub room: Room,
    pub peers: HashMap<PeerId, PeerInfo>,
    pub cmd_tx: mpsc::Sender<RoomCommand>,
}

#[derive(Default)]
pub struct RoomRegistry {
    rooms: DashMap<RoomId, RoomHandle>,
}

impl RoomRegistry {
    pub fn insert(&self, room: Room, cmd_tx: mpsc::Sender<RoomCommand>) {
        self.rooms.insert(
            room.id,
            RoomHandle { room, peers: HashMap::new(), cmd_tx },
        );
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
        self.rooms.get(&id).map(|h| h.peers.len()).unwrap_or(0)
    }
}

use serde::{Deserialize, Serialize};
use crate::models::PeerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined { peer_id: PeerId },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    PeerJoined { peer_id: PeerId },
    PeerLeft { peer_id: PeerId },
    Error { reason: String },
}

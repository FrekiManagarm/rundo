use serde::{Deserialize, Serialize};
use crate::models::PeerId;

/// Messages sent from browser → server over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Browser's SDP answer to the server's initial or renegotiation offer.
    Answer { sdp: String },
    /// Trickle ICE candidate from the browser.
    IceCandidate { candidate: String },
    /// Text chat message.
    ChatMessage { text: String },
}

/// Messages sent from server → browser over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Initial join: contains the server's SDP offer + the assigned peer ID.
    Joined { peer_id: PeerId, sdp: String },
    /// Renegotiation offer from the server (new peer joined / left).
    Offer { sdp: String },
    /// ICE candidate from the server (trickle, sent after initial offer).
    IceCandidate { candidate: String },
    /// Another peer joined the room (UI notification only).
    PeerJoined { peer_id: PeerId },
    /// Another peer left the room (UI notification only).
    PeerLeft { peer_id: PeerId },
    /// Chat message from another peer.
    ChatFrom { from_peer: PeerId, text: String, timestamp_ms: i64 },
    /// Server-side error.
    Error { reason: String },
}

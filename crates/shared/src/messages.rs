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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn client_leave_serializes_correctly() {
        let msg = ClientMessage::Leave;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"leave"}"#);
    }

    #[test]
    fn client_offer_serializes_correctly() {
        let msg = ClientMessage::Offer { sdp: "v=0".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "offer");
        assert_eq!(parsed["sdp"], "v=0");
    }

    #[test]
    fn server_error_serializes_correctly() {
        let msg = ServerMessage::Error { reason: "bad".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["reason"], "bad");
    }

    #[test]
    fn server_joined_roundtrip() {
        let peer_id = PeerId(Uuid::new_v4());
        let msg = ServerMessage::Joined { peer_id };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "joined");
    }
}

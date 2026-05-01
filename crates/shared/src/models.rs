use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub Uuid);

impl UserId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for UserId {
    fn default() -> Self { Self::new() }
}

impl RoomId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for RoomId {
    fn default() -> Self { Self::new() }
}

impl PeerId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for PeerId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    Conference,
    Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub kind: RoomKind,
    pub owner_id: UserId,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub user_id: UserId,
    pub connected_at: DateTime<Utc>,
}

# WebRTC Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a production-ready WebRTC SFU server (Rust: Axum + str0m) with a Next.js + shadcn/ui client supporting video conferencing and live streaming.

**Architecture:** Cargo workspace (`server` + `shared` crates) + Next.js app. Each `Room` runs in its own Tokio task. A single shared UDP socket handles all media; a demultiplexer routes incoming packets to the correct `Peer` task by source `SocketAddr`. Signaling (SDP + ICE) travels over WebSocket. For simplicity, complete SDP is exchanged (non-trickle ICE: wait for `icegatheringstate === 'complete'` in the browser before sending the offer).

**Tech Stack:** Rust (axum 0.8, str0m, tokio, jsonwebtoken, argon2, dashmap, tracing, thiserror, anyhow), Next.js 15 App Router (TypeScript, Tailwind CSS, shadcn/ui)

---

## File Map

```
webrtc-server/
├── Cargo.toml                          ← workspace
├── crates/
│   ├── shared/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  ← re-exports
│   │       ├── models.rs               ← RoomId, PeerId, UserId, RoomKind, User, Room, PeerInfo
│   │       └── messages.rs             ← ClientMessage, ServerMessage
│   └── server/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                 ← entry point, router, AppState init
│           ├── config.rs               ← Config struct
│           ├── error.rs                ← AppError + IntoResponse
│           ├── state.rs                ← AppState
│           ├── auth/
│           │   ├── mod.rs
│           │   ├── password.rs         ← hash_password / verify_password
│           │   ├── jwt.rs              ← Claims / encode_jwt / decode_jwt
│           │   ├── handlers.rs         ← POST /auth/register, POST /auth/token
│           │   └── middleware.rs       ← AuthUser extractor
│           ├── store/
│           │   ├── mod.rs              ← Store trait
│           │   └── memory.rs           ← InMemoryStore
│           ├── rooms/
│           │   ├── mod.rs
│           │   ├── registry.rs         ← RoomRegistry (live state + channel to Room task)
│           │   ├── room.rs             ← Room Tokio task + SFU fanout loop
│           │   ├── peer.rs             ← Peer (str0m Rtc + channels)
│           │   └── handlers.rs         ← REST handlers
│           ├── signaling/
│           │   ├── mod.rs
│           │   ├── handler.rs          ← WebSocket upgrade handler
│           │   └── session.rs          ← per-peer signaling loop
│           └── sfu/
│               ├── mod.rs
│               └── udp.rs              ← shared UdpSocket + demux task
└── client/
    ├── app/
    │   ├── layout.tsx
    │   ├── page.tsx                    ← redirect to /rooms if logged in
    │   ├── (auth)/
    │   │   ├── login/page.tsx
    │   │   └── register/page.tsx
    │   └── rooms/
    │       ├── page.tsx                ← room list + create dialog
    │       └── [id]/page.tsx           ← room page
    ├── components/
    │   ├── VideoGrid.tsx
    │   ├── VideoTile.tsx
    │   ├── Controls.tsx
    │   └── CreateRoomDialog.tsx
    ├── hooks/
    │   ├── useWebRTC.ts
    │   └── useAuth.ts
    └── lib/
        ├── api.ts
        ├── ws.ts
        └── storage.ts
```

---

## Phase 1 — Rust Server

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/shared/Cargo.toml`
- Create: `crates/shared/src/lib.rs`
- Create: `crates/server/Cargo.toml`
- Create: `crates/server/src/main.rs`

- [ ] **Step 1: Write workspace Cargo.toml**

```toml
# Cargo.toml
[workspace]
members = ["crates/server", "crates/shared"]
resolver = "2"

[profile.release]
opt-level = 3
lto = true
```

- [ ] **Step 2: Write shared crate manifest**

```toml
# crates/shared/Cargo.toml
[package]
name = "shared"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: Write shared crate lib.rs (empty for now)**

```rust
// crates/shared/src/lib.rs
pub mod messages;
pub mod models;
```

- [ ] **Step 4: Create placeholder modules**

```rust
// crates/shared/src/models.rs
// (empty — filled in Task 2)
```

```rust
// crates/shared/src/messages.rs
// (empty — filled in Task 2)
```

- [ ] **Step 5: Write server crate manifest**

```toml
# crates/server/Cargo.toml
[package]
name = "server"
version = "0.1.0"
edition = "2024"

[dependencies]
shared = { path = "../shared" }
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
str0m = "0.7"
jsonwebtoken = "9"
argon2 = "0.5"
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
anyhow = "1"
thiserror = "2"
dashmap = "6"
tower-http = { version = "0.6", features = ["cors"] }
chrono = { version = "0.4", features = ["serde"] }
futures-util = "0.3"
rand = "0.8"

[dev-dependencies]
axum-test = "16"
```

- [ ] **Step 6: Write minimal main.rs**

```rust
// crates/server/src/main.rs
#[tokio::main]
async fn main() {
    println!("WebRTC server starting…");
}
```

- [ ] **Step 7: Verify the workspace compiles**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/
git commit -m "feat: scaffold cargo workspace with server and shared crates"
```

---

### Task 2: Shared types

**Files:**
- Modify: `crates/shared/src/models.rs`
- Modify: `crates/shared/src/messages.rs`

- [ ] **Step 1: Write models.rs**

```rust
// crates/shared/src/models.rs
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
impl RoomId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}
impl PeerId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
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
```

- [ ] **Step 2: Write messages.rs**

```rust
// crates/shared/src/messages.rs
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
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p shared
```
Expected: 0 failures (no tests yet, just verify it compiles).

- [ ] **Step 4: Commit**

```bash
git add crates/shared/
git commit -m "feat: add shared types (models, messages)"
```

---

### Task 3: Config + AppError

**Files:**
- Create: `crates/server/src/config.rs`
- Create: `crates/server/src/error.rs`

- [ ] **Step 1: Write config.rs**

```rust
// crates/server/src/config.rs
#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub udp_media_port: u16,
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4000),
            udp_media_port: std::env::var("UDP_MEDIA_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4001),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-prod".to_string()),
        }
    }
}
```

- [ ] **Step 2: Write error.rs**

```rust
// crates/server/src/error.rs
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("room is full")]
    RoomFull,
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            AppError::RoomFull => (StatusCode::CONFLICT, "room_full", self.to_string()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "an internal error occurred".to_string(),
            ),
        };
        (status, Json(json!({ "error": code, "message": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [ ] **Step 3: Write tests for AppError responses**

Add inside `crates/server/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn unauthorized_returns_401() {
        let resp = AppError::Unauthorized.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn not_found_returns_404() {
        let resp = AppError::NotFound("room xyz".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn forbidden_returns_403() {
        let resp = AppError::Forbidden.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p server error
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/config.rs crates/server/src/error.rs
git commit -m "feat: add Config and AppError types"
```

---

### Task 4: Store trait + InMemoryStore

**Files:**
- Create: `crates/server/src/store/mod.rs`
- Create: `crates/server/src/store/memory.rs`

- [ ] **Step 1: Write the Store trait**

```rust
// crates/server/src/store/mod.rs
pub mod memory;

use anyhow::Result;
use shared::models::{Room, RoomId, User, UserId};

#[allow(async_fn_in_trait)]
pub trait Store: Send + Sync {
    async fn create_user(&self, user: User) -> Result<()>;
    async fn get_user_by_email(&self, email: &str) -> Option<User>;
    async fn get_user_by_id(&self, id: UserId) -> Option<User>;
    async fn create_room(&self, room: Room) -> Result<()>;
    async fn get_room(&self, id: RoomId) -> Option<Room>;
    async fn delete_room(&self, id: RoomId) -> Result<()>;
    async fn list_rooms(&self) -> Vec<Room>;
}
```

- [ ] **Step 2: Write InMemoryStore**

```rust
// crates/server/src/store/memory.rs
use anyhow::Result;
use dashmap::DashMap;
use shared::models::{Room, RoomId, User, UserId};
use crate::store::Store;

#[derive(Default)]
pub struct InMemoryStore {
    users_by_id: DashMap<UserId, User>,
    users_by_email: DashMap<String, UserId>,
    rooms: DashMap<RoomId, Room>,
}

impl Store for InMemoryStore {
    async fn create_user(&self, user: User) -> Result<()> {
        self.users_by_email.insert(user.email.clone(), user.id);
        self.users_by_id.insert(user.id, user);
        Ok(())
    }

    async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let id = self.users_by_email.get(email)?.clone();
        self.users_by_id.get(&id).map(|u| u.clone())
    }

    async fn get_user_by_id(&self, id: UserId) -> Option<User> {
        self.users_by_id.get(&id).map(|u| u.clone())
    }

    async fn create_room(&self, room: Room) -> Result<()> {
        self.rooms.insert(room.id, room);
        Ok(())
    }

    async fn get_room(&self, id: RoomId) -> Option<Room> {
        self.rooms.get(&id).map(|r| r.clone())
    }

    async fn delete_room(&self, id: RoomId) -> Result<()> {
        self.rooms.remove(&id);
        Ok(())
    }

    async fn list_rooms(&self) -> Vec<Room> {
        self.rooms.iter().map(|r| r.clone()).collect()
    }
}
```

- [ ] **Step 3: Write tests for InMemoryStore**

Add at the bottom of `crates/server/src/store/memory.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::{RoomKind, UserId, RoomId};
    use chrono::Utc;

    fn make_user() -> User {
        User {
            id: UserId::new(),
            email: "alice@example.com".to_string(),
            password_hash: "hash".to_string(),
            created_at: Utc::now(),
        }
    }

    fn make_room(owner_id: UserId) -> Room {
        Room {
            id: RoomId::new(),
            name: "Test Room".to_string(),
            kind: RoomKind::Conference,
            owner_id,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_and_get_user() {
        let store = InMemoryStore::default();
        let user = make_user();
        let email = user.email.clone();
        store.create_user(user.clone()).await.unwrap();
        let found = store.get_user_by_email(&email).await.unwrap();
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn get_unknown_user_returns_none() {
        let store = InMemoryStore::default();
        assert!(store.get_user_by_email("nobody@example.com").await.is_none());
    }

    #[tokio::test]
    async fn create_and_list_rooms() {
        let store = InMemoryStore::default();
        let owner = UserId::new();
        let room = make_room(owner);
        store.create_room(room.clone()).await.unwrap();
        let rooms = store.list_rooms().await;
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, room.id);
    }

    #[tokio::test]
    async fn delete_room() {
        let store = InMemoryStore::default();
        let room = make_room(UserId::new());
        let id = room.id;
        store.create_room(room).await.unwrap();
        store.delete_room(id).await.unwrap();
        assert!(store.get_room(id).await.is_none());
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p server store
```
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/store/
git commit -m "feat: add Store trait and InMemoryStore"
```

---

### Task 5: AppState + health endpoint

**Files:**
- Create: `crates/server/src/state.rs`
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Write state.rs**

```rust
// crates/server/src/state.rs
use std::sync::Arc;
use crate::{config::Config, store::Store};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<dyn Store>,
}

impl AppState {
    pub fn new(config: Config, store: impl Store + 'static) -> Self {
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
        }
    }
}
```

- [ ] **Step 2: Write main.rs with health route**

```rust
// crates/server/src/main.rs
mod auth;
mod config;
mod error;
mod rooms;
mod signaling;
mod sfu;
mod state;
mod store;

use axum::{routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use store::memory::InMemoryStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();

    let config = config::Config::from_env();
    let store = InMemoryStore::default();
    let state = AppState::new(config.clone(), store);

    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", config.http_port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
```

- [ ] **Step 3: Add stub modules so it compiles**

```rust
// crates/server/src/auth/mod.rs
// (empty for now)
```
```rust
// crates/server/src/rooms/mod.rs
// (empty for now)
```
```rust
// crates/server/src/signaling/mod.rs
// (empty for now)
```
```rust
// crates/server/src/sfu/mod.rs
// (empty for now)
```

- [ ] **Step 4: Build and smoke-test**

```bash
cargo build -p server
cargo run -p server &
sleep 1
curl -s http://localhost:4000/health
# Expected: {"status":"ok"}
kill %1
```

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/
git commit -m "feat: add AppState and health endpoint"
```

---

### Task 6: Auth — password hashing

**Files:**
- Create: `crates/server/src/auth/password.rs`
- Modify: `crates/server/src/auth/mod.rs`

- [ ] **Step 1: Write failing tests first**

```rust
// crates/server/src/auth/password.rs
use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash error: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("parse hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_correct_password() {
        let hash = hash_password("secret123").unwrap();
        assert!(verify_password("secret123", &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_password("secret123").unwrap();
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn same_password_produces_different_hashes() {
        let h1 = hash_password("secret").unwrap();
        let h2 = hash_password("secret").unwrap();
        assert_ne!(h1, h2);
    }
}
```

- [ ] **Step 2: Export from auth/mod.rs**

```rust
// crates/server/src/auth/mod.rs
pub mod password;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p server auth::password
```
Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/server/src/auth/
git commit -m "feat: add password hashing with argon2"
```

---

### Task 7: Auth — JWT

**Files:**
- Create: `crates/server/src/auth/jwt.rs`
- Modify: `crates/server/src/auth/mod.rs`

- [ ] **Step 1: Write jwt.rs with tests**

```rust
// crates/server/src/auth/jwt.rs
use anyhow::{anyhow, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use shared::models::UserId;

const EXPIRY_SECS: u64 = 86_400; // 24 h

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: UserId,
    pub exp: u64,
}

pub fn encode_jwt(user_id: UserId, secret: &str) -> Result<String> {
    let exp = jsonwebtoken::get_current_timestamp() + EXPIRY_SECS;
    let claims = Claims { sub: user_id, exp };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("jwt encode: {e}"))
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow!("jwt decode: {e}"))?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::UserId;

    const SECRET: &str = "test-secret";

    #[test]
    fn encode_and_decode_roundtrip() {
        let id = UserId::new();
        let token = encode_jwt(id, SECRET).unwrap();
        let claims = decode_jwt(&token, SECRET).unwrap();
        assert_eq!(claims.sub, id);
    }

    #[test]
    fn wrong_secret_fails_decode() {
        let token = encode_jwt(UserId::new(), SECRET).unwrap();
        assert!(decode_jwt(&token, "wrong-secret").is_err());
    }

    #[test]
    fn tampered_token_fails_decode() {
        let mut token = encode_jwt(UserId::new(), SECRET).unwrap();
        token.push('x');
        assert!(decode_jwt(&token, SECRET).is_err());
    }
}
```

- [ ] **Step 2: Export from auth/mod.rs**

```rust
// crates/server/src/auth/mod.rs
pub mod jwt;
pub mod password;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p server auth::jwt
```
Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/server/src/auth/jwt.rs crates/server/src/auth/mod.rs
git commit -m "feat: add JWT encode/decode"
```

---

### Task 8: Auth — HTTP handlers + middleware

**Files:**
- Create: `crates/server/src/auth/handlers.rs`
- Create: `crates/server/src/auth/middleware.rs`
- Modify: `crates/server/src/auth/mod.rs`
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Write auth handlers**

```rust
// crates/server/src/auth/handlers.rs
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::models::{User, UserId};

use crate::{
    auth::{jwt::encode_jwt, password::{hash_password, verify_password}},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: UserId,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    if state.store.get_user_by_email(&body.email).await.is_some() {
        return Err(AppError::NotFound("email already registered".to_string()));
    }
    let password_hash = hash_password(&body.password)
        .map_err(|e| AppError::Internal(e))?;
    let user = User {
        id: UserId::new(),
        email: body.email,
        password_hash,
        created_at: Utc::now(),
    };
    let token = encode_jwt(user.id, &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(e))?;
    let user_id = user.id;
    state.store.create_user(user).await.map_err(AppError::Internal)?;
    Ok(Json(AuthResponse { token, user_id }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let user = state
        .store
        .get_user_by_email(&body.email)
        .await
        .ok_or(AppError::Unauthorized)?;
    let valid = verify_password(&body.password, &user.password_hash)
        .map_err(|e| AppError::Internal(e))?;
    if !valid {
        return Err(AppError::Unauthorized);
    }
    let token = encode_jwt(user.id, &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(e))?;
    Ok(Json(AuthResponse { token, user_id: user.id }))
}
```

- [ ] **Step 2: Write AuthUser extractor (middleware.rs)**

```rust
// crates/server/src/auth/middleware.rs
use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::{request::Parts, HeaderMap},
    RequestPartsExt,
};
use serde::Deserialize;
use shared::models::UserId;

use crate::{auth::jwt::decode_jwt, error::AppError, state::AppState};

#[derive(Debug, Clone)]
pub struct AuthUser(pub UserId);

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Try Bearer header
        let token = extract_bearer(&parts.headers)
            // 2. Fall back to ?token= query param (for WebSocket)
            .or_else(|| {
                parts
                    .extract::<Query<TokenQuery>>()
                    .ok()
                    .and_then(|q| q.0.token)
            })
            .ok_or(AppError::Unauthorized)?;

        let claims = decode_jwt(&token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser(claims.sub))
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("Authorization")?.to_str().ok()?;
    auth.strip_prefix("Bearer ").map(|s| s.to_string())
}
```

- [ ] **Step 3: Update auth/mod.rs**

```rust
// crates/server/src/auth/mod.rs
pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod password;

use axum::{routing::post, Router};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/token", post(handlers::login))
}
```

- [ ] **Step 4: Wire auth router into main.rs**

Replace the `Router::new()` block in `main.rs`:

```rust
let app = Router::new()
    .route("/health", get(health_handler))
    .nest("/auth", auth::router())
    .with_state(state)
    .layer(CorsLayer::permissive());
```

- [ ] **Step 5: Build and smoke-test**

```bash
cargo build -p server
cargo run -p server &
sleep 1

# Register
curl -s -X POST http://localhost:4000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@test.com","password":"pass123"}' | jq .

# Login
curl -s -X POST http://localhost:4000/auth/token \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@test.com","password":"pass123"}' | jq .

# Wrong password
curl -s -X POST http://localhost:4000/auth/token \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@test.com","password":"wrong"}' | jq .
# Expected: {"error":"unauthorized",...}

kill %1
```

- [ ] **Step 6: Commit**

```bash
git add crates/server/src/auth/ crates/server/src/main.rs
git commit -m "feat: add auth register/login endpoints with JWT"
```

---

### Task 9: Rooms — registry + REST handlers

**Files:**
- Create: `crates/server/src/rooms/registry.rs`
- Create: `crates/server/src/rooms/handlers.rs`
- Modify: `crates/server/src/rooms/mod.rs`
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Write registry.rs**

```rust
// crates/server/src/rooms/registry.rs
use dashmap::DashMap;
use shared::models::{PeerId, PeerInfo, Room, RoomId, RoomRecord};
use std::collections::HashMap;
use tokio::sync::mpsc;

// Message types for communicating with a Room task
#[derive(Debug)]
pub enum RoomCommand {
    PeerJoined {
        peer_id: PeerId,
        info: PeerInfo,
        // sender to forward RTP data to peer's signaling session
        rtp_tx: mpsc::Sender<RtpPayload>,
    },
    PeerLeft { peer_id: PeerId },
    MediaData { from: PeerId, payload: RtpPayload },
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
```

Note: `RoomRecord` needs to be added to `shared/src/models.rs` (see below).

- [ ] **Step 2: Add RoomRecord to shared models**

Append to `crates/shared/src/models.rs`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomRecord {
    pub room: Option<Room>,
    pub peers: HashMap<PeerId, PeerInfo>,
}
```

- [ ] **Step 3: Write rooms/handlers.rs**

```rust
// crates/server/src/rooms/handlers.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::models::{Room, RoomId, RoomKind};

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub kind: RoomKind,
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub id: RoomId,
    pub name: String,
    pub kind: RoomKind,
    pub owner_id: shared::models::UserId,
    pub peer_count: usize,
}

impl RoomResponse {
    fn from(room: Room, peer_count: usize) -> Self {
        Self {
            id: room.id,
            name: room.name,
            kind: room.kind,
            owner_id: room.owner_id,
            peer_count,
        }
    }
}

pub async fn create_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreateRoomRequest>,
) -> AppResult<(StatusCode, Json<RoomResponse>)> {
    let room = Room {
        id: RoomId::new(),
        name: body.name,
        kind: body.kind,
        owner_id: user_id,
        created_at: Utc::now(),
    };
    state.store.create_room(room.clone()).await.map_err(AppError::Internal)?;

    // Spawn the room's Tokio task
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(256);
    state.registry.insert(room.clone(), cmd_tx);
    tokio::spawn(crate::rooms::room::run_room(room.id, cmd_rx));

    let resp = RoomResponse::from(room, 0);
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn list_rooms(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
) -> AppResult<Json<Vec<RoomResponse>>> {
    let rooms = state.store.list_rooms().await;
    let responses: Vec<_> = rooms
        .into_iter()
        .map(|r| {
            let count = state.registry.peer_count(r.id);
            RoomResponse::from(r, count)
        })
        .collect();
    Ok(Json(responses))
}

pub async fn get_room(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(room_id): Path<RoomId>,
) -> AppResult<Json<RoomResponse>> {
    let room = state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;
    let count = state.registry.peer_count(room_id);
    Ok(Json(RoomResponse::from(room, count)))
}

pub async fn delete_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<RoomId>,
) -> AppResult<StatusCode> {
    let room = state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;
    if room.owner_id != user_id {
        return Err(AppError::Forbidden);
    }
    state.store.delete_room(room_id).await.map_err(AppError::Internal)?;
    state.registry.remove(room_id);
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 4: Update rooms/mod.rs**

```rust
// crates/server/src/rooms/mod.rs
pub mod handlers;
pub mod peer;
pub mod registry;
pub mod room;

use axum::{
    routing::{delete, get, post},
    Router,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_room).get(handlers::list_rooms))
        .route("/:id", get(handlers::get_room).delete(handlers::delete_room))
        .route("/:id/join", get(crate::signaling::handler::ws_handler))
}
```

Add stub files that will be filled in later:

```rust
// crates/server/src/rooms/peer.rs
// (filled in Task 11)
```

```rust
// crates/server/src/rooms/room.rs
use shared::models::RoomId;
use tokio::sync::mpsc;
use crate::rooms::registry::RoomCommand;

pub async fn run_room(_room_id: RoomId, mut cmd_rx: mpsc::Receiver<RoomCommand>) {
    while let Some(_cmd) = cmd_rx.recv().await {
        // SFU fanout — implemented in Task 12
    }
}
```

- [ ] **Step 5: Add registry to AppState**

Update `crates/server/src/state.rs`:

```rust
// crates/server/src/state.rs
use std::sync::Arc;
use crate::{config::Config, rooms::registry::RoomRegistry, store::Store};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<dyn Store>,
    pub registry: Arc<RoomRegistry>,
}

impl AppState {
    pub fn new(config: Config, store: impl Store + 'static) -> Self {
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
            registry: Arc::new(RoomRegistry::default()),
        }
    }
}
```

- [ ] **Step 6: Wire rooms router into main.rs**

Update the `Router::new()` block:

```rust
let app = Router::new()
    .route("/health", get(health_handler))
    .nest("/auth", auth::router())
    .nest("/rooms", rooms::router())
    .with_state(state)
    .layer(CorsLayer::permissive());
```

Also add `use crate::rooms;` at the top of `main.rs` if not already there.

- [ ] **Step 7: Add stub signaling handler so it compiles**

```rust
// crates/server/src/signaling/handler.rs
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use crate::state::AppState;

pub async fn ws_handler(
    _ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> Response {
    todo!("implemented in Task 13")
}
```

```rust
// crates/server/src/signaling/mod.rs
pub mod handler;
pub mod session;
```

```rust
// crates/server/src/signaling/session.rs
// (filled in Task 13)
```

- [ ] **Step 8: Build and smoke-test**

```bash
cargo build -p server
cargo run -p server &
sleep 1
TOKEN=$(curl -s -X POST http://localhost:4000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"bob@test.com","password":"pass"}' | jq -r .token)

curl -s -X POST http://localhost:4000/rooms \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"My Room","kind":"conference"}' | jq .

curl -s http://localhost:4000/rooms \
  -H "Authorization: Bearer $TOKEN" | jq .

kill %1
```

- [ ] **Step 9: Commit**

```bash
git add crates/
git commit -m "feat: add rooms registry and REST CRUD endpoints"
```

---

### Task 10: SFU — UDP socket + demultiplexer

**Files:**
- Create: `crates/server/src/sfu/udp.rs`
- Modify: `crates/server/src/sfu/mod.rs`
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Write udp.rs**

```rust
// crates/server/src/sfu/udp.rs
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};

// Routes raw UDP packets to the correct peer task by source address
pub struct UdpDemux {
    socket: Arc<UdpSocket>,
    // source addr → sender to peer task
    routes: HashMap<SocketAddr, mpsc::Sender<(SocketAddr, Vec<u8>)>>,
}

impl UdpDemux {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket, routes: HashMap::new() }
    }

    pub fn register(&mut self, addr: SocketAddr, tx: mpsc::Sender<(SocketAddr, Vec<u8>)>) {
        self.routes.insert(addr, tx);
    }

    pub fn unregister(&mut self, addr: &SocketAddr) {
        self.routes.remove(addr);
    }

    pub fn socket(&self) -> Arc<UdpSocket> {
        self.socket.clone()
    }
}

// Runs the UDP demux loop in a Tokio task.
// Returns a sender to register/unregister peer routes at runtime.
pub enum DemuxControl {
    Register { addr: SocketAddr, tx: mpsc::Sender<(SocketAddr, Vec<u8>)> },
    Unregister { addr: SocketAddr },
}

pub async fn run_demux(
    socket: Arc<UdpSocket>,
    mut ctrl_rx: mpsc::Receiver<DemuxControl>,
) {
    let mut routes: HashMap<SocketAddr, mpsc::Sender<(SocketAddr, Vec<u8>)>> = HashMap::new();
    let mut buf = vec![0u8; 65535];

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, src)) => {
                        let data = buf[..len].to_vec();
                        if let Some(tx) = routes.get(&src) {
                            let _ = tx.try_send((src, data));
                        }
                    }
                    Err(e) => tracing::error!("UDP recv error: {e}"),
                }
            }
            ctrl = ctrl_rx.recv() => {
                match ctrl {
                    Some(DemuxControl::Register { addr, tx }) => {
                        routes.insert(addr, tx);
                    }
                    Some(DemuxControl::Unregister { addr }) => {
                        routes.remove(&addr);
                    }
                    None => break,
                }
            }
        }
    }
}
```

- [ ] **Step 2: Update sfu/mod.rs**

```rust
// crates/server/src/sfu/mod.rs
pub mod udp;

use std::{net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};
use udp::{DemuxControl, run_demux};

#[derive(Clone)]
pub struct SfuState {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub demux_ctrl: mpsc::Sender<DemuxControl>,
}

impl SfuState {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(addr).await?);
        let local_addr = socket.local_addr()?;
        let (ctrl_tx, ctrl_rx) = mpsc::channel(256);
        tokio::spawn(run_demux(socket.clone(), ctrl_rx));
        Ok(Self { socket, local_addr, demux_ctrl: ctrl_tx })
    }
}
```

- [ ] **Step 3: Add SfuState to AppState**

Update `crates/server/src/state.rs`:

```rust
use crate::{config::Config, rooms::registry::RoomRegistry, sfu::SfuState, store::Store};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<dyn Store>,
    pub registry: Arc<RoomRegistry>,
    pub sfu: SfuState,
}

impl AppState {
    pub async fn new(config: Config, store: impl Store + 'static) -> anyhow::Result<Self> {
        let sfu_addr = format!("0.0.0.0:{}", config.udp_media_port);
        let sfu = SfuState::bind(&sfu_addr).await?;
        Ok(Self {
            config: Arc::new(config),
            store: Arc::new(store),
            registry: Arc::new(RoomRegistry::default()),
            sfu,
        })
    }
}
```

- [ ] **Step 4: Update main.rs to use async AppState::new**

```rust
// In main() replace:
//   let state = AppState::new(config.clone(), store);
// With:
let state = AppState::new(config.clone(), store).await
    .expect("failed to initialize SFU state");
```

- [ ] **Step 5: Build**

```bash
cargo build -p server
```
Expected: compiles cleanly.

- [ ] **Step 6: Commit**

```bash
git add crates/server/src/sfu/ crates/server/src/state.rs crates/server/src/main.rs
git commit -m "feat: add shared UDP socket and demux task for SFU"
```

---

### Task 11: Peer — str0m integration

**Files:**
- Modify: `crates/server/src/rooms/peer.rs`

- [ ] **Step 1: Write peer.rs**

```rust
// crates/server/src/rooms/peer.rs
use std::{net::SocketAddr, sync::Arc, time::Instant};
use anyhow::Result;
use shared::models::PeerId;
use str0m::{Event, Input, Output, Rtc, RtcConfig, change::SdpOffer, net::Receive};
use tokio::{net::UdpSocket, sync::mpsc};

use crate::{
    rooms::registry::{RoomCommand, RtpPayload},
    sfu::udp::DemuxControl,
};

pub struct Peer {
    pub id: PeerId,
    rtc: Rtc,
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl Peer {
    pub fn new(
        id: PeerId,
        socket: Arc<UdpSocket>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        let rtc = RtcConfig::default()
            .set_ice_lite(true)  // server-side: ICE lite mode
            .build();
        Self { id, rtc, socket, local_addr, remote_addr }
    }

    pub fn accept_offer(&mut self, sdp: &str) -> Result<String> {
        let offer = SdpOffer::from_sdp_string(sdp)?;
        let mut api = self.rtc.sdp_api();
        let answer = api.accept_offer(offer)?;
        api.apply();
        Ok(answer.to_sdp_string())
    }
}

// Runs a Peer's event loop in its own Tokio task.
// Forwards media data to the Room via room_cmd_tx.
// Receives raw UDP packets from the demux via udp_rx.
pub async fn run_peer(
    peer_id: PeerId,
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    sdp_offer: String,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
    demux_ctrl: mpsc::Sender<DemuxControl>,
    // Receives UDP packets from demux
    mut udp_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    // Receives RTP payloads to write to this peer (from other peers via SFU)
    mut rtp_rx: mpsc::Receiver<RtpPayload>,
    // Sends the SDP answer back to the signaling session
    answer_tx: tokio::sync::oneshot::Sender<Result<String>>,
) {
    let mut peer = Peer::new(peer_id, socket.clone(), local_addr, remote_addr);

    // Accept the SDP offer
    let answer = match peer.accept_offer(&sdp_offer) {
        Ok(a) => a,
        Err(e) => {
            let _ = answer_tx.send(Err(e));
            return;
        }
    };
    let _ = answer_tx.send(Ok(answer));

    // Register with demux
    let _ = demux_ctrl
        .send(DemuxControl::Register {
            addr: remote_addr,
            tx: {
                // We channel packets through an internal sender — udp_rx already connected
                // Actually the caller passes udp_rx/tx pair; we just use udp_rx here.
                // (see signaling/session.rs for how udp_tx is passed to DemuxControl::Register)
                // This field is unused here; registration is done before calling run_peer
                todo!("see signaling session for wiring")
            },
        })
        .await;

    // Main loop
    loop {
        // Compute timeout from str0m
        let timeout = match peer.rtc.poll_output() {
            Ok(Output::Timeout(t)) => t,
            Ok(Output::Transmit(t)) => {
                let _ = socket.send_to(&t.contents, t.destination).await;
                continue;
            }
            Ok(Output::Event(Event::MediaData(data))) => {
                let payload = RtpPayload {
                    data: data.data.to_vec(),
                    timestamp: data.time.as_nanos() as u32,
                    payload_type: data.pt.into(),
                };
                let _ = room_cmd_tx
                    .send(RoomCommand::MediaData { from: peer_id, payload })
                    .await;
                continue;
            }
            Ok(Output::Event(_)) => continue,
            Err(e) => {
                tracing::warn!("peer {peer_id:?} rtc error: {e}");
                break;
            }
        };

        let now = Instant::now();
        let sleep_dur = timeout.saturating_duration_since(now);

        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {
                let _ = peer.rtc.handle_input(Input::Timeout(Instant::now()));
            }
            Some((src, data)) = udp_rx.recv() => {
                let input = Input::Receive(
                    Instant::now(),
                    Receive {
                        source: src,
                        destination: local_addr,
                        contents: data.into(),
                    },
                );
                let _ = peer.rtc.handle_input(input);
            }
            Some(rtp) = rtp_rx.recv() => {
                // Forward incoming RTP from other peers to this peer's browser
                // Requires a negotiated send track — simplified for now
                tracing::debug!("peer {peer_id:?} received forwarded RTP ({} bytes)", rtp.data.len());
            }
        }
    }

    let _ = demux_ctrl
        .send(DemuxControl::Unregister { addr: remote_addr })
        .await;
    let _ = room_cmd_tx.send(RoomCommand::PeerLeft { peer_id }).await;
}
```

- [ ] **Step 2: Build (expect warnings, not errors)**

```bash
cargo build -p server 2>&1 | grep -v "^warning"
```
Expected: no errors (some warnings about `todo!` and unused are fine).

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/rooms/peer.rs
git commit -m "feat: add Peer struct with str0m Rtc integration"
```

---

### Task 12: Room Tokio task + SFU fanout

**Files:**
- Modify: `crates/server/src/rooms/room.rs`

- [ ] **Step 1: Write room.rs**

```rust
// crates/server/src/rooms/room.rs
use std::collections::HashMap;
use shared::models::{PeerId, RoomId};
use tokio::sync::mpsc;

use crate::rooms::registry::{RoomCommand, RtpPayload};

// Holds the per-peer RTP sender so the Room can forward media
struct PeerHandle {
    rtp_tx: mpsc::Sender<RtpPayload>,
}

// Runs the Room's SFU fanout loop.
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
                    tracing::info!("room {room_id:?} is empty, shutting down");
                    break;
                }
            }

            RoomCommand::MediaData { from, payload } => {
                // SFU fanout: send to all other peers
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
```

- [ ] **Step 2: Build**

```bash
cargo build -p server 2>&1 | grep -v "^warning"
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/rooms/room.rs
git commit -m "feat: add Room Tokio task with SFU RTP fanout"
```

---

### Task 13: Signaling — WebSocket handler

**Files:**
- Modify: `crates/server/src/signaling/handler.rs`
- Create: `crates/server/src/signaling/session.rs`

- [ ] **Step 1: Write signaling/handler.rs**

```rust
// crates/server/src/signaling/handler.rs
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use shared::models::RoomId;

use crate::{
    auth::middleware::AuthUser,
    error::AppError,
    state::AppState,
};

#[derive(Deserialize)]
pub struct JoinQuery {
    pub token: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<RoomId>,
    AuthUser(user_id): AuthUser,
) -> Result<Response, AppError> {
    // Verify room exists
    state
        .store
        .get_room(room_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("room {room_id:?}")))?;

    let cmd_tx = state
        .registry
        .get_cmd_tx(room_id)
        .ok_or_else(|| AppError::NotFound("room not active".to_string()))?;

    let sfu = state.sfu.clone();

    Ok(ws.on_upgrade(move |socket| {
        crate::signaling::session::run_session(socket, user_id, room_id, cmd_tx, sfu)
    }))
}
```

- [ ] **Step 2: Write signaling/session.rs**

```rust
// crates/server/src/signaling/session.rs
use std::net::SocketAddr;
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{
    messages::{ClientMessage, ServerMessage},
    models::{PeerId, PeerInfo, UserId, RoomId},
};
use tokio::sync::{mpsc, oneshot};

use crate::{
    rooms::registry::{RoomCommand, RtpPayload},
    sfu::{SfuState, udp::DemuxControl},
};

pub async fn run_session(
    socket: WebSocket,
    user_id: UserId,
    room_id: RoomId,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
    sfu: SfuState,
) {
    let peer_id = PeerId::new();
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send Joined message
    let joined = ServerMessage::Joined { peer_id };
    if send_msg(&mut ws_tx, &joined).await.is_err() {
        return;
    }

    // Wait for the SDP offer from the browser
    let offer_sdp = loop {
        match ws_rx.next().await {
            Some(Ok(Message::Text(text))) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Offer { sdp }) => break sdp,
                    Ok(ClientMessage::Leave) | None => return,
                    _ => continue,
                }
            }
            Some(Ok(Message::Close(_))) | None => return,
            _ => continue,
        }
    };

    // The browser's remote address — in a real deployment, extract from the HTTP request.
    // For local dev, we use a placeholder; str0m will update once STUN arrives.
    let remote_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Channel: demux → peer task (raw UDP)
    let (udp_tx, udp_rx) = mpsc::channel::<(SocketAddr, Vec<u8>)>(256);
    // Channel: room → peer (forwarded RTP)
    let (rtp_tx, rtp_rx) = mpsc::channel::<RtpPayload>(256);
    // One-shot for SDP answer
    let (answer_tx, answer_rx) = oneshot::channel::<anyhow::Result<String>>();

    // Register with demux (skip for 127.0.0.1:0 placeholder)
    if remote_addr.port() != 0 {
        let _ = sfu
            .demux_ctrl
            .send(DemuxControl::Register { addr: remote_addr, tx: udp_tx })
            .await;
    }

    // Notify room that peer joined
    let peer_info = PeerInfo { peer_id, user_id, connected_at: Utc::now() };
    let _ = room_cmd_tx
        .send(RoomCommand::PeerJoined { peer_id, info: peer_info, rtp_tx })
        .await;

    // Spawn the peer's str0m task
    tokio::spawn(crate::rooms::peer::run_peer(
        peer_id,
        sfu.socket.clone(),
        sfu.local_addr,
        remote_addr,
        offer_sdp,
        room_cmd_tx.clone(),
        sfu.demux_ctrl.clone(),
        udp_rx,
        rtp_rx,
        answer_tx,
    ));

    // Wait for SDP answer and send it back
    match answer_rx.await {
        Ok(Ok(sdp)) => {
            let msg = ServerMessage::Answer { sdp };
            if send_msg(&mut ws_tx, &msg).await.is_err() {
                return;
            }
        }
        _ => return,
    }

    // Forward remaining WS messages to peer / room
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
```

- [ ] **Step 3: Fix run_peer signature (remove the todo!)**

In `crates/server/src/rooms/peer.rs`, remove the `DemuxControl::Register` block inside `run_peer` (the demux registration is now done in `session.rs` before spawning). Replace the `todo!` block:

```rust
// Remove this entire block from run_peer:
// let _ = demux_ctrl
//     .send(DemuxControl::Register { ... })
//     .await;
```

And update the function signature to remove the unused `demux_ctrl` parameter, or keep it for the `Unregister` at the end. Simplified version:

```rust
pub async fn run_peer(
    peer_id: PeerId,
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    sdp_offer: String,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
    demux_ctrl: mpsc::Sender<DemuxControl>,
    mut udp_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    mut rtp_rx: mpsc::Receiver<RtpPayload>,
    answer_tx: tokio::sync::oneshot::Sender<Result<String>>,
) {
    let mut peer = Peer::new(peer_id, socket.clone(), local_addr, remote_addr);

    let answer = match peer.accept_offer(&sdp_offer) {
        Ok(a) => a,
        Err(e) => { let _ = answer_tx.send(Err(e)); return; }
    };
    let _ = answer_tx.send(Ok(answer));

    loop {
        let timeout = match peer.rtc.poll_output() {
            Ok(Output::Timeout(t)) => t,
            Ok(Output::Transmit(t)) => {
                let _ = socket.send_to(&t.contents, t.destination).await;
                continue;
            }
            Ok(Output::Event(Event::MediaData(data))) => {
                let payload = RtpPayload {
                    data: data.data.to_vec(),
                    timestamp: data.time.as_nanos() as u32,
                    payload_type: data.pt.into(),
                };
                let _ = room_cmd_tx.send(RoomCommand::MediaData { from: peer_id, payload }).await;
                continue;
            }
            Ok(Output::Event(_)) => continue,
            Err(e) => { tracing::warn!("peer {peer_id:?} error: {e}"); break; }
        };

        let sleep_dur = timeout.saturating_duration_since(Instant::now());
        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {
                let _ = peer.rtc.handle_input(Input::Timeout(Instant::now()));
            }
            Some((src, data)) = udp_rx.recv() => {
                let _ = peer.rtc.handle_input(Input::Receive(
                    Instant::now(),
                    Receive { source: src, destination: local_addr, contents: data.into() },
                ));
            }
            Some(rtp) = rtp_rx.recv() => {
                tracing::debug!("forward RTP {} bytes to {peer_id:?}", rtp.data.len());
            }
        }
    }

    let _ = demux_ctrl.send(DemuxControl::Unregister { addr: remote_addr }).await;
    let _ = room_cmd_tx.send(RoomCommand::PeerLeft { peer_id }).await;
}
```

- [ ] **Step 4: Build**

```bash
cargo build -p server 2>&1 | grep -E "^error"
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/signaling/ crates/server/src/rooms/peer.rs
git commit -m "feat: add WebSocket signaling handler and peer session"
```

---

### Task 14: Integration test + final wiring

**Files:**
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Final main.rs**

```rust
// crates/server/src/main.rs
mod auth;
mod config;
mod error;
mod rooms;
mod signaling;
mod sfu;
mod state;
mod store;

use axum::{routing::get, Json, Router};
use serde_json::json;
use state::AppState;
use store::memory::InMemoryStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();

    let config = config::Config::from_env();
    let store = InMemoryStore::default();
    let state = AppState::new(config.clone(), store)
        .await
        .expect("failed to init AppState");

    let app = Router::new()
        .route("/health", get(health_handler))
        .nest("/auth", auth::router())
        .nest("/rooms", rooms::router())
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", config.http_port);
    tracing::info!("HTTP listening on {addr}");
    tracing::info!("UDP media on 0.0.0.0:{}", config.udp_media_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
```

- [ ] **Step 2: Run all server tests**

```bash
cargo test -p server
cargo test -p shared
```
Expected: all pass.

- [ ] **Step 3: Full smoke test**

```bash
cargo run -p server &
sleep 2

# Register + login
TOKEN=$(curl -s -X POST http://localhost:4000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.com","password":"pass123"}' | jq -r .token)

# Create room
ROOM=$(curl -s -X POST http://localhost:4000/rooms \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Room","kind":"conference"}' | jq -r .id)

echo "Room ID: $ROOM"

# List rooms
curl -s http://localhost:4000/rooms \
  -H "Authorization: Bearer $TOKEN" | jq .

# Health
curl -s http://localhost:4000/health

kill %1
```
Expected: token generated, room created, room listed, health ok.

- [ ] **Step 4: Commit**

```bash
git add crates/server/src/main.rs
git commit -m "feat: wire all server modules into main router"
```

---

## Phase 2 — Next.js Client

### Task 15: Next.js + Tailwind + shadcn/ui setup

**Files:**
- Create: `client/` (via CLI)

- [ ] **Step 1: Scaffold Next.js app**

```bash
cd /path/to/webrtc-server
npx create-next-app@latest client \
  --typescript \
  --tailwind \
  --eslint \
  --app \
  --no-src-dir \
  --import-alias "@/*"
cd client
```

- [ ] **Step 2: Init shadcn/ui**

```bash
npx shadcn@latest init
```
When prompted:
- Style: **Default**
- Base color: **Slate**
- CSS variables: **Yes**

- [ ] **Step 3: Install required shadcn components**

```bash
npx shadcn@latest add button card input label dialog badge separator
```

- [ ] **Step 4: Add axios for HTTP requests**

```bash
npm install axios
```

- [ ] **Step 5: Start dev server and verify**

```bash
npm run dev &
sleep 3
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000
# Expected: 200
kill %1
```

- [ ] **Step 6: Commit**

```bash
cd ..
git add client/
git commit -m "feat: scaffold Next.js app with Tailwind and shadcn/ui"
```

---

### Task 16: Storage + API client + auth hook

**Files:**
- Create: `client/lib/storage.ts`
- Create: `client/lib/api.ts`
- Create: `client/hooks/useAuth.ts`

- [ ] **Step 1: Write storage.ts**

```typescript
// client/lib/storage.ts
const TOKEN_KEY = "webrtc_token";
const USER_ID_KEY = "webrtc_user_id";

export const storage = {
  getToken: () => localStorage.getItem(TOKEN_KEY),
  setToken: (token: string) => localStorage.setItem(TOKEN_KEY, token),
  clearToken: () => localStorage.removeItem(TOKEN_KEY),
  getUserId: () => localStorage.getItem(USER_ID_KEY),
  setUserId: (id: string) => localStorage.setItem(USER_ID_KEY, id),
  clear: () => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_ID_KEY);
  },
};
```

- [ ] **Step 2: Write api.ts**

```typescript
// client/lib/api.ts
import axios from "axios";
import { storage } from "./storage";

const BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";

export const api = axios.create({ baseURL: BASE_URL });

api.interceptors.request.use((config) => {
  const token = storage.getToken();
  if (token) config.headers.Authorization = `Bearer ${token}`;
  return config;
});

// Types
export type RoomKind = "conference" | "stream";

export interface AuthResponse {
  token: string;
  user_id: string;
}

export interface Room {
  id: string;
  name: string;
  kind: RoomKind;
  owner_id: string;
  peer_count: number;
}

// Auth
export const authApi = {
  register: (email: string, password: string) =>
    api.post<AuthResponse>("/auth/register", { email, password }),
  login: (email: string, password: string) =>
    api.post<AuthResponse>("/auth/token", { email, password }),
};

// Rooms
export const roomsApi = {
  list: () => api.get<Room[]>("/rooms"),
  create: (name: string, kind: RoomKind) =>
    api.post<Room>("/rooms", { name, kind }),
  get: (id: string) => api.get<Room>(`/rooms/${id}`),
  delete: (id: string) => api.delete(`/rooms/${id}`),
};
```

- [ ] **Step 3: Write useAuth.ts**

```typescript
// client/hooks/useAuth.ts
"use client";
import { useState, useCallback, useEffect } from "react";
import { authApi } from "@/lib/api";
import { storage } from "@/lib/storage";

export function useAuth() {
  const [token, setToken] = useState<string | null>(null);
  const [userId, setUserId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setToken(storage.getToken());
    setUserId(storage.getUserId());
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    setLoading(true);
    setError(null);
    try {
      const { data } = await authApi.login(email, password);
      storage.setToken(data.token);
      storage.setUserId(data.user_id);
      setToken(data.token);
      setUserId(data.user_id);
    } catch {
      setError("Invalid email or password");
    } finally {
      setLoading(false);
    }
  }, []);

  const register = useCallback(async (email: string, password: string) => {
    setLoading(true);
    setError(null);
    try {
      const { data } = await authApi.register(email, password);
      storage.setToken(data.token);
      storage.setUserId(data.user_id);
      setToken(data.token);
      setUserId(data.user_id);
    } catch {
      setError("Registration failed. Email may already be in use.");
    } finally {
      setLoading(false);
    }
  }, []);

  const logout = useCallback(() => {
    storage.clear();
    setToken(null);
    setUserId(null);
  }, []);

  return { token, userId, loading, error, login, register, logout, isAuth: !!token };
}
```

- [ ] **Step 4: Verify TypeScript**

```bash
cd client && npx tsc --noEmit && cd ..
```
Expected: no type errors.

- [ ] **Step 5: Commit**

```bash
git add client/lib/ client/hooks/useAuth.ts
git commit -m "feat: add API client, storage helpers, and useAuth hook"
```

---

### Task 17: Auth pages

**Files:**
- Create: `client/app/(auth)/login/page.tsx`
- Create: `client/app/(auth)/register/page.tsx`
- Modify: `client/app/page.tsx`

- [ ] **Step 1: Write login page**

```tsx
// client/app/(auth)/login/page.tsx
"use client";
import { useRouter } from "next/navigation";
import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/hooks/useAuth";

export default function LoginPage() {
  const { login, loading, error } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    await login(email, password);
    if (!error) router.push("/rooms");
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-slate-950 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle className="text-2xl">Sign in</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-1">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {error && <p className="text-sm text-red-500">{error}</p>}
            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? "Signing in…" : "Sign in"}
            </Button>
            <p className="text-center text-sm text-slate-400">
              No account?{" "}
              <Link href="/register" className="underline">
                Register
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
```

- [ ] **Step 2: Write register page**

```tsx
// client/app/(auth)/register/page.tsx
"use client";
import { useRouter } from "next/navigation";
import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/hooks/useAuth";

export default function RegisterPage() {
  const { register, loading, error } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    await register(email, password);
    if (!error) router.push("/rooms");
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-slate-950 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle className="text-2xl">Create account</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-1">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                minLength={6}
                required
              />
            </div>
            {error && <p className="text-sm text-red-500">{error}</p>}
            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? "Creating account…" : "Create account"}
            </Button>
            <p className="text-center text-sm text-slate-400">
              Already have an account?{" "}
              <Link href="/login" className="underline">
                Sign in
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
```

- [ ] **Step 3: Update root page.tsx to redirect**

```tsx
// client/app/page.tsx
import { redirect } from "next/navigation";

export default function Home() {
  redirect("/rooms");
}
```

- [ ] **Step 4: TypeScript check**

```bash
cd client && npx tsc --noEmit && cd ..
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add client/app/
git commit -m "feat: add login and register pages"
```

---

### Task 18: Rooms page + CreateRoomDialog

**Files:**
- Create: `client/components/CreateRoomDialog.tsx`
- Create: `client/app/rooms/page.tsx`

- [ ] **Step 1: Write CreateRoomDialog.tsx**

```tsx
// client/components/CreateRoomDialog.tsx
"use client";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { roomsApi, RoomKind } from "@/lib/api";

interface Props {
  onCreated: () => void;
}

export function CreateRoomDialog({ onCreated }: Props) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<RoomKind>("conference");
  const [loading, setLoading] = useState(false);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    try {
      await roomsApi.create(name, kind);
      setOpen(false);
      setName("");
      onCreated();
    } finally {
      setLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>New Room</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create a room</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleCreate} className="space-y-4 pt-2">
          <div className="space-y-1">
            <Label htmlFor="room-name">Room name</Label>
            <Input
              id="room-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </div>
          <div className="space-y-1">
            <Label>Type</Label>
            <div className="flex gap-2">
              {(["conference", "stream"] as RoomKind[]).map((k) => (
                <button
                  key={k}
                  type="button"
                  onClick={() => setKind(k)}
                  className={`flex-1 rounded border px-3 py-2 text-sm capitalize transition-colors ${
                    kind === k
                      ? "border-blue-500 bg-blue-500/10 text-blue-400"
                      : "border-slate-700 text-slate-400 hover:border-slate-500"
                  }`}
                >
                  {k}
                </button>
              ))}
            </div>
          </div>
          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? "Creating…" : "Create"}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 2: Write rooms/page.tsx**

```tsx
// client/app/rooms/page.tsx
"use client";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { CreateRoomDialog } from "@/components/CreateRoomDialog";
import { roomsApi, Room } from "@/lib/api";
import { storage } from "@/lib/storage";

export default function RoomsPage() {
  const [rooms, setRooms] = useState<Room[]>([]);
  const [loading, setLoading] = useState(true);
  const router = useRouter();

  useEffect(() => {
    if (!storage.getToken()) { router.push("/login"); return; }
    fetchRooms();
  }, [router]);

  async function fetchRooms() {
    setLoading(true);
    try {
      const { data } = await roomsApi.list();
      setRooms(data);
    } finally {
      setLoading(false);
    }
  }

  function logout() {
    storage.clear();
    router.push("/login");
  }

  return (
    <main className="min-h-screen bg-slate-950 p-6 text-white">
      <div className="mx-auto max-w-3xl">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold">Rooms</h1>
          <div className="flex gap-2">
            <CreateRoomDialog onCreated={fetchRooms} />
            <Button variant="ghost" onClick={logout}>
              Sign out
            </Button>
          </div>
        </div>

        {loading ? (
          <p className="text-slate-400">Loading…</p>
        ) : rooms.length === 0 ? (
          <p className="text-slate-400">No rooms yet. Create one to get started.</p>
        ) : (
          <div className="grid gap-3">
            {rooms.map((room) => (
              <Link key={room.id} href={`/rooms/${room.id}`}>
                <Card className="cursor-pointer transition-colors hover:bg-slate-800">
                  <CardHeader className="pb-2">
                    <CardTitle className="flex items-center justify-between text-base">
                      {room.name}
                      <Badge variant="outline" className="capitalize">
                        {room.kind}
                      </Badge>
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-slate-400">
                      {room.peer_count} participant
                      {room.peer_count !== 1 ? "s" : ""}
                    </p>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
```

- [ ] **Step 3: TypeScript check**

```bash
cd client && npx tsc --noEmit && cd ..
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add client/components/CreateRoomDialog.tsx client/app/rooms/page.tsx
git commit -m "feat: add rooms list page and create room dialog"
```

---

### Task 19: useWebRTC hook

**Files:**
- Create: `client/lib/ws.ts`
- Create: `client/hooks/useWebRTC.ts`

- [ ] **Step 1: Write ws.ts**

```typescript
// client/lib/ws.ts
import { ServerMessage, ClientMessage } from "./types";

export class SignalingClient {
  private ws: WebSocket;
  private handlers: Map<string, (msg: ServerMessage) => void> = new Map();

  constructor(url: string, onMessage: (msg: ServerMessage) => void) {
    this.ws = new WebSocket(url);
    this.ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data) as ServerMessage;
        onMessage(msg);
      } catch {}
    };
  }

  send(msg: ClientMessage) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  onOpen(cb: () => void) { this.ws.onopen = cb; }
  onClose(cb: () => void) { this.ws.onclose = cb; }
  onError(cb: (e: Event) => void) { this.ws.onerror = cb; }

  close() { this.ws.close(); }
}
```

- [ ] **Step 2: Write types.ts**

```typescript
// client/lib/types.ts
export type ClientMessage =
  | { type: "offer"; sdp: string }
  | { type: "answer"; sdp: string }
  | { type: "ice_candidate"; candidate: string }
  | { type: "leave" };

export type ServerMessage =
  | { type: "joined"; peer_id: string }
  | { type: "answer"; sdp: string }
  | { type: "ice_candidate"; candidate: string }
  | { type: "peer_joined"; peer_id: string }
  | { type: "peer_left"; peer_id: string }
  | { type: "error"; reason: string };
```

- [ ] **Step 3: Write useWebRTC.ts**

```typescript
// client/hooks/useWebRTC.ts
"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { SignalingClient } from "@/lib/ws";
import { ServerMessage } from "@/lib/types";
import { storage } from "@/lib/storage";

const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";
const WS_URL = API_URL.replace(/^http/, "ws");

const ICE_SERVERS: RTCIceServer[] = [{ urls: "stun:stun.l.google.com:19302" }];

export interface RemotePeer {
  id: string;
  stream: MediaStream | null;
}

export function useWebRTC(roomId: string) {
  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [peers, setPeers] = useState<RemotePeer[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pc = useRef<RTCPeerConnection | null>(null);
  const sigRef = useRef<SignalingClient | null>(null);
  const localStreamRef = useRef<MediaStream | null>(null);

  const handleServerMessage = useCallback((msg: ServerMessage) => {
    switch (msg.type) {
      case "joined":
        // Start WebRTC negotiation
        startNegotiation();
        break;
      case "answer":
        pc.current?.setRemoteDescription({ type: "answer", sdp: msg.sdp });
        break;
      case "ice_candidate":
        pc.current?.addIceCandidate({ candidate: msg.candidate });
        break;
      case "peer_joined":
        setPeers((prev) =>
          prev.find((p) => p.id === msg.peer_id)
            ? prev
            : [...prev, { id: msg.peer_id, stream: null }]
        );
        break;
      case "peer_left":
        setPeers((prev) => prev.filter((p) => p.id !== msg.peer_id));
        break;
      case "error":
        setError(msg.reason);
        break;
    }
  }, []);

  async function startNegotiation() {
    if (!pc.current || !localStreamRef.current) return;

    // Add local tracks
    for (const track of localStreamRef.current.getTracks()) {
      pc.current.addTrack(track, localStreamRef.current);
    }

    // Create offer — wait for ICE gathering to complete (non-trickle)
    const offer = await pc.current.createOffer();
    await pc.current.setLocalDescription(offer);

    await new Promise<void>((resolve) => {
      if (!pc.current) return resolve();
      if (pc.current.iceGatheringState === "complete") return resolve();
      pc.current.onicegatheringstatechange = () => {
        if (pc.current?.iceGatheringState === "complete") resolve();
      };
    });

    // Send complete SDP (with all ICE candidates embedded)
    sigRef.current?.send({
      type: "offer",
      sdp: pc.current.localDescription!.sdp,
    });
  }

  const join = useCallback(async () => {
    try {
      // Get user media
      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });
      localStreamRef.current = stream;
      setLocalStream(stream);

      // Create peer connection
      const conn = new RTCPeerConnection({ iceServers: ICE_SERVERS });
      pc.current = conn;

      // Handle incoming tracks from server (forwarded from other peers)
      conn.ontrack = (e) => {
        const [remoteStream] = e.streams;
        setPeers((prev) => {
          // Assign stream to the most recent peer without one
          const idx = prev.findIndex((p) => !p.stream);
          if (idx === -1) return prev;
          const updated = [...prev];
          updated[idx] = { ...updated[idx], stream: remoteStream };
          return updated;
        });
      };

      // Connect signaling
      const token = storage.getToken();
      const wsUrl = `${WS_URL}/rooms/${roomId}/join?token=${token}`;
      const sig = new SignalingClient(wsUrl, handleServerMessage);
      sigRef.current = sig;

      sig.onOpen(() => setConnected(true));
      sig.onClose(() => setConnected(false));
      sig.onError(() => setError("WebSocket connection failed"));
    } catch (e) {
      setError("Could not access camera/microphone");
    }
  }, [roomId, handleServerMessage]);

  const leave = useCallback(() => {
    sigRef.current?.send({ type: "leave" });
    sigRef.current?.close();
    pc.current?.close();
    localStreamRef.current?.getTracks().forEach((t) => t.stop());
    setLocalStream(null);
    setPeers([]);
    setConnected(false);
  }, []);

  useEffect(() => {
    return () => { leave(); };
  }, [leave]);

  return { localStream, peers, connected, error, join, leave };
}
```

- [ ] **Step 4: TypeScript check**

```bash
cd client && npx tsc --noEmit && cd ..
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add client/hooks/useWebRTC.ts client/lib/ws.ts client/lib/types.ts
git commit -m "feat: add useWebRTC hook with WebRTC + signaling logic"
```

---

### Task 20: Room page + VideoGrid + Controls

**Files:**
- Create: `client/components/VideoTile.tsx`
- Create: `client/components/VideoGrid.tsx`
- Create: `client/components/Controls.tsx`
- Create: `client/app/rooms/[id]/page.tsx`

- [ ] **Step 1: Write VideoTile.tsx**

```tsx
// client/components/VideoTile.tsx
"use client";
import { useEffect, useRef } from "react";
import { Badge } from "@/components/ui/badge";

interface Props {
  stream: MediaStream | null;
  label: string;
  muted?: boolean;
}

export function VideoTile({ stream, label, muted = false }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (videoRef.current && stream) {
      videoRef.current.srcObject = stream;
    }
  }, [stream]);

  return (
    <div className="relative overflow-hidden rounded-lg bg-slate-800 aspect-video">
      {stream ? (
        <video
          ref={videoRef}
          autoPlay
          playsInline
          muted={muted}
          className="h-full w-full object-cover"
        />
      ) : (
        <div className="flex h-full items-center justify-center text-slate-500">
          No video
        </div>
      )}
      <Badge className="absolute bottom-2 left-2 bg-black/60">
        {label}
      </Badge>
    </div>
  );
}
```

- [ ] **Step 2: Write VideoGrid.tsx**

```tsx
// client/components/VideoGrid.tsx
import { VideoTile } from "./VideoTile";
import { RemotePeer } from "@/hooks/useWebRTC";

interface Props {
  localStream: MediaStream | null;
  peers: RemotePeer[];
}

export function VideoGrid({ localStream, peers }: Props) {
  const total = 1 + peers.length;
  const cols = total <= 1 ? 1 : total <= 4 ? 2 : 3;

  return (
    <div
      className="grid gap-2 p-2"
      style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}
    >
      <VideoTile stream={localStream} label="You" muted />
      {peers.map((peer) => (
        <VideoTile key={peer.id} stream={peer.stream} label={peer.id.slice(0, 8)} />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Write Controls.tsx**

```tsx
// client/components/Controls.tsx
"use client";
import { useState } from "react";
import { Button } from "@/components/ui/button";

interface Props {
  localStream: MediaStream | null;
  onLeave: () => void;
}

export function Controls({ localStream, onLeave }: Props) {
  const [audioEnabled, setAudioEnabled] = useState(true);
  const [videoEnabled, setVideoEnabled] = useState(true);

  function toggleAudio() {
    if (!localStream) return;
    const enabled = !audioEnabled;
    localStream.getAudioTracks().forEach((t) => (t.enabled = enabled));
    setAudioEnabled(enabled);
  }

  function toggleVideo() {
    if (!localStream) return;
    const enabled = !videoEnabled;
    localStream.getVideoTracks().forEach((t) => (t.enabled = enabled));
    setVideoEnabled(enabled);
  }

  return (
    <div className="flex items-center justify-center gap-3 bg-slate-900 p-4">
      <Button
        variant={audioEnabled ? "secondary" : "destructive"}
        onClick={toggleAudio}
      >
        {audioEnabled ? "Mute" : "Unmute"}
      </Button>
      <Button
        variant={videoEnabled ? "secondary" : "destructive"}
        onClick={toggleVideo}
      >
        {videoEnabled ? "Stop video" : "Start video"}
      </Button>
      <Button variant="destructive" onClick={onLeave}>
        Leave
      </Button>
    </div>
  );
}
```

- [ ] **Step 4: Write room/[id]/page.tsx**

```tsx
// client/app/rooms/[id]/page.tsx
"use client";
import { useParams, useRouter } from "next/navigation";
import { useEffect } from "react";
import { VideoGrid } from "@/components/VideoGrid";
import { Controls } from "@/components/Controls";
import { useWebRTC } from "@/hooks/useWebRTC";
import { storage } from "@/lib/storage";

export default function RoomPage() {
  const { id } = useParams<{ id: string }>();
  const router = useRouter();
  const { localStream, peers, connected, error, join, leave } = useWebRTC(id);

  useEffect(() => {
    if (!storage.getToken()) { router.push("/login"); return; }
    join();
    return () => { leave(); };
  }, [join, leave, router]);

  function handleLeave() {
    leave();
    router.push("/rooms");
  }

  return (
    <main className="flex h-screen flex-col bg-slate-950 text-white">
      <header className="flex items-center justify-between border-b border-slate-800 px-4 py-2">
        <h1 className="font-semibold">Room</h1>
        <span className="text-sm text-slate-400">
          {connected ? `${peers.length + 1} participant(s)` : "Connecting…"}
        </span>
      </header>

      {error && (
        <div className="bg-red-900/40 px-4 py-2 text-sm text-red-400">{error}</div>
      )}

      <div className="flex-1 overflow-auto">
        <VideoGrid localStream={localStream} peers={peers} />
      </div>

      <Controls localStream={localStream} onLeave={handleLeave} />
    </main>
  );
}
```

- [ ] **Step 5: TypeScript check**

```bash
cd client && npx tsc --noEmit && cd ..
```
Expected: no errors.

- [ ] **Step 6: Start both server and client, verify UI**

```bash
# Terminal 1
cargo run -p server

# Terminal 2
cd client && npm run dev
```

Open http://localhost:3000 in a browser:
- Should redirect to `/rooms`
- Register an account → land on rooms list
- Create a room → room appears in list
- Click a room → room page loads with camera feed

- [ ] **Step 7: Commit**

```bash
git add client/components/ client/app/rooms/[id]/
git commit -m "feat: add room page with VideoGrid and Controls"
```

---

## Done

The server exposes REST + WebSocket endpoints with auth, room management, and str0m-driven WebRTC. The Next.js client handles auth, room listing/creation, and in-browser WebRTC with camera/mic. Next steps when ready:

- Replace `InMemoryStore` with `PostgresStore` (add `sqlx`, run migrations)
- Implement proper SDP renegotiation in the SFU for track forwarding between peers
- Add TURN server support for production NAT traversal

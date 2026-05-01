# WebRTC Server — Design Spec

**Date:** 2026-05-01  
**Stack:** Rust (Axum + str0m) + Next.js  
**Approche:** Monolithe modulaire

---

## Contexte & objectifs

Serveur WebRTC production-ready supportant :
- **Visioconférence multi-participants** (SFU — Selective Forwarding Unit)
- **Streaming unidirectionnel** (diffusion live vers des viewers)
- **Scale cible** : centaines/milliers de participants simultanés
- **Client** : application Next.js (App Router)

---

## Architecture générale

```
┌─────────────────────────────────────────────────────┐
│                   Axum HTTP Server                  │
│                                                     │
│  POST /auth/register     → auth module              │
│  POST /auth/token        → auth module              │
│  POST /rooms             → rooms module             │
│  GET  /rooms/:id/join    → signaling (upgrade WS)   │
│  GET  /health            → healthcheck              │
└──────────────────┬──────────────────────────────────┘
                   │
          ┌────────▼────────┐
          │  AppState (Arc) │  ← partagé entre tous les handlers
          │  - RoomRegistry │
          │  - AuthService  │
          │  - Store        │
          └────────┬────────┘
                   │
     ┌─────────────▼──────────────┐
     │        RoomRegistry        │  ← HashMap<RoomId, Room>
     │  - créer / lister / join   │
     └─────────────┬──────────────┘
                   │
     ┌─────────────▼──────────────┐
     │           Room             │
     │  - peers: HashMap<PeerId, Peer>        │
     │  - boucle SFU (Tokio task) │
     └─────────────┬──────────────┘
                   │
     ┌─────────────▼──────────────┐
     │        Peer / str0m        │
     │  - Rtc (str0m)             │
     │  - WebSocket signaling     │
     │  - canal média optionnel   │
     └────────────────────────────┘
```

Chaque `Room` tourne dans sa propre tâche Tokio, pilotée par un `mpsc::channel`. La boucle SFU lit les paquets RTP d'un peer et les retransmet à tous les autres peers de la room (fanout). Le pipeline média optionnel reçoit une copie via un `broadcast::channel` sans bloquer le routing principal.

---

## Structure du projet

```
webrtc-server/
├── Cargo.toml                  ← workspace
├── crates/
│   ├── server/                 ← serveur Axum + SFU
│   │   └── src/
│   │       ├── main.rs
│   │       ├── auth/           ← JWT generation/validation
│   │       ├── rooms/          ← RoomRegistry, Room, Peer
│   │       ├── signaling/      ← handlers WebSocket
│   │       ├── sfu/            ← routing RTP via str0m
│   │       ├── media/          ← pipeline optionnel (recording...)
│   │       └── store/          ← trait Store + impl InMemory + (Postgres)
│   └── shared/                 ← types partagés (messages WS, DTOs)
│       └── src/lib.rs
└── client/                     ← app Next.js (App Router + TypeScript)
    ├── app/
    │   ├── page.tsx            ← home, créer/rejoindre une room
    │   ├── room/[id]/page.tsx  ← page de visioconférence
    │   └── layout.tsx
    ├── components/
    │   ├── VideoGrid.tsx       ← grille des flux vidéo
    │   └── Controls.tsx        ← mute, caméra, quitter
    ├── hooks/
    │   └── useWebRTC.ts        ← logique WebRTC + signalisation WS
    ├── lib/
    │   └── api.ts              ← appels vers le serveur Rust
    └── package.json
```

---

## Protocole de signalisation (WebSocket)

**Flux d'établissement :**

```
Client                          Serveur
  │── POST /auth/token ────────► │  ← credentials → JWT
  │◄── { token } ───────────────│
  │── POST /rooms ─────────────► │  ← créer une room
  │◄── { room_id } ─────────────│
  │── GET /rooms/:id/join (WS) ► │  ← upgrade WebSocket (JWT en header)
  │◄── { type: "joined", peer_id }
  │── { type: "offer", sdp } ──► │  ← SDP offer
  │◄── { type: "answer", sdp } ─│  ← SDP answer de str0m
  │── { type: "ice", candidate } │  ← trickle ICE (bidirectionnel)
  │    ════ flux RTP/RTCP ══════ │  ← média établi (UDP)
  │◄── { type: "peer_joined" } ──│  ← événements room
  │◄── { type: "peer_left" } ───│
```

**Types de messages (crate `shared`) :**

```rust
enum ClientMessage {
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    Leave,
}

enum ServerMessage {
    Joined { peer_id: PeerId },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    PeerJoined { peer_id: PeerId },
    PeerLeft { peer_id: PeerId },
    Error { reason: String },
}
```

---

## Auth & gestion des rooms

**Authentification — JWT stateless**

- `POST /auth/register` — crée un utilisateur (argon2 pour le hash du mot de passe)
- `POST /auth/token` — valide les credentials, retourne un JWT signé (HS256, expiry 24h)
- Middleware Axum extrait et valide le JWT sur toutes les routes protégées

**Endpoints rooms :**

| Endpoint | Auth | Description |
|---|---|---|
| `POST /rooms` | ✓ | Créer une room (`conference` ou `stream`) |
| `GET /rooms` | ✓ | Lister les rooms disponibles |
| `GET /rooms/:id` | ✓ | Détails d'une room |
| `DELETE /rooms/:id` | ✓ owner | Fermer une room |
| `GET /rooms/:id/join` | ✓ | Rejoindre via WebSocket |

**Modèles de données :**

```rust
struct User {
    id: UserId,
    email: String,
    password_hash: String,
    created_at: DateTime<Utc>,
}

struct Room {
    id: RoomId,
    name: String,
    kind: RoomKind,       // Conference | Stream
    owner_id: UserId,
    created_at: DateTime<Utc>,
}

struct PeerInfo {
    peer_id: PeerId,
    user_id: UserId,
    connected_at: DateTime<Utc>,
}

struct RoomRecord {       // état live, géré par RoomRegistry en mémoire
    room: Room,
    peers: HashMap<PeerId, PeerInfo>,
}
```

---

## Couche de stockage (Store trait)

```rust
trait Store: Send + Sync {
    async fn create_user(&self, user: User) -> Result<()>;
    async fn get_user_by_email(&self, email: &str) -> Option<User>;
    async fn get_room(&self, id: RoomId) -> Option<Room>;   // données persistantes uniquement
    async fn create_room(&self, room: Room) -> Result<()>;
    async fn delete_room(&self, id: RoomId) -> Result<()>;
    async fn list_rooms(&self) -> Vec<Room>;
}
// L'état live (RoomRecord avec les peers) est géré par RoomRegistry en mémoire,
// indépendamment du Store.
```

Deux implémentations prévues :
- **`InMemoryStore`** — `DashMap` pour la concurrence, démarrage immédiat, pas de persistence
- **`PostgresStore`** — SQLx + migrations, tables `users` et `rooms`, switch via config sans changer les handlers

---

## Gestion des erreurs

Type `AppError` centralisé implémentant `IntoResponse` :

```rust
enum AppError {
    Unauthorized,
    NotFound(String),
    RoomFull,
    Forbidden,
    Internal(anyhow::Error),
}
// Réponse JSON : { "error": "room_not_found", "message": "Room abc123 not found" }
```

---

## Observabilité

- Logs structurés via `tracing` + `tracing-subscriber` (JSON en prod, pretty en dev)
- Spans sur chaque connexion WebRTC, chaque room, chaque opération SFU
- `GET /health` retourne le statut du serveur, le nombre de rooms et peers actifs

---

## Tests

- **Unitaires** — logique de room, validation JWT, routing SFU avec faux paquets RTP
- **Intégration** — cycle complet REST + WebSocket avec `axum::test` et store in-memory
- **Client** — validation manuelle dans le browser dans un premier temps

---

## Dépendances principales

**Rust (server) :**
- `axum` — HTTP server + WebSocket
- `str0m` — stack WebRTC pure Rust
- `tokio` — runtime async
- `serde` / `serde_json` — sérialisation
- `jsonwebtoken` — JWT
- `argon2` — hash de mots de passe
- `uuid` — identifiants
- `tracing` / `tracing-subscriber` — logs
- `anyhow` — gestion des erreurs
- `dashmap` — HashMap concurrent pour l'in-memory store
- `sqlx` (feature-gated) — PostgreSQL

**Next.js (client) :**
- TypeScript + App Router
- Tailwind CSS — styling
- shadcn/ui — composants UI (Button, Card, Dialog, Input, Badge...)
- WebRTC browser API (natif, pas de lib tierce)

---

## Évolutions futures

- **PostgreSQL** — activer `PostgresStore` via variable d'environnement `DATABASE_URL`
- **Scale horizontal** — remplacer `InMemoryStore` par Redis pour l'état partagé entre instances
- **Enregistrement** — pipeline média branché sur le `broadcast::channel` de la room
- **Refresh tokens** — étendre l'auth JWT

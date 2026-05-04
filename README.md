# Rundo

A self-hosted video conferencing application. Rundo connects participants directly browser-to-browser via WebRTC, using a lightweight Rust signaling server to coordinate the connection setup.

## How it works

```
Browser A ──WebSocket──► Rust signaling server ◄──WebSocket── Browser B
    │                                                               │
    └───────────────── WebRTC (audio + video) ─────────────────────┘
```

1. Authenticated users create or join rooms through the Next.js frontend.
2. The Rust server handles the signaling exchange (SDP offers/answers and ICE candidates) over WebSocket.
3. Once peers have negotiated a connection, media flows directly between browsers — the server is no longer in the media path.

## Tech stack

| Layer | Technology |
|---|---|
| Backend | Rust · Axum · Tokio · SQLx |
| Database | SQLite (dev) · PostgreSQL (prod) |
| Auth | JWT · Argon2 |
| Frontend | Next.js 16 · React 19 · TypeScript |
| Styling | Tailwind CSS 4 · shadcn/ui |
| Real-time | WebSocket (signaling) · WebRTC (media) |

## Features

- Register and log in with email/password (Argon2-hashed, JWT-issued)
- Create rooms of type **conference** (everyone talks) or **stream** (broadcast-style)
- Full mesh WebRTC: each participant connects directly to every other participant
- Screen sharing: replace your camera feed with a screen capture, restore it on stop
- In-room text chat: send messages to all participants via the signaling WebSocket; the server timestamps and broadcasts each message
- Live peer count visible on the rooms list
- SQLite out of the box — swap to PostgreSQL for production with a single env var

## Prerequisites

- [Rust](https://rustup.rs) (stable, edition 2024)
- [Bun](https://bun.sh) (or npm/yarn)

## Getting started

### 1. Start the server

```bash
cargo run -p server
```

The server listens on `http://localhost:4000` by default.

### 2. Start the frontend

```bash
cd client
bun install
bun dev
```

Open `http://localhost:3000`, register an account, and create a room.

## Environment variables

### Server

| Variable | Default | Description |
|---|---|---|
| `HTTP_PORT` | `4000` | Port for the HTTP/WebSocket server |
| `UDP_MEDIA_PORT` | `4001` | Reserved UDP port for future media relay |
| `UDP_MEDIA_HOST` | `127.0.0.1` | Advertised host in SDP candidates — set to your LAN IP or domain for remote peers |
| `JWT_SECRET` | `dev-secret-change-in-prod` | Secret used to sign and verify JWTs — **must be changed in production** |
| `DATABASE_URL` | `sqlite://rundo.db` | SQLite file path or full `postgres://user:pass@host/db` URL |

### Frontend

Create `client/.env.local`:

```env
NEXT_PUBLIC_API_URL=http://localhost:4000
NEXT_PUBLIC_WS_URL=ws://localhost:4000
```

## API reference

### Auth

| Method | Path | Body | Description |
|---|---|---|---|
| `POST` | `/auth/register` | `{ email, password }` | Create an account, returns `{ token, user_id }` |
| `POST` | `/auth/login` | `{ email, password }` | Authenticate, returns `{ token, user_id }` |

### Rooms

All room endpoints require `Authorization: Bearer <token>`.

| Method | Path | Description |
|---|---|---|
| `GET` | `/rooms` | List all rooms |
| `POST` | `/rooms` | Create a room (`{ name, kind: "conference" \| "stream" }`) |
| `GET` | `/rooms/:id` | Get a single room |
| `DELETE` | `/rooms/:id` | Delete a room |
| `GET` | `/rooms/:id/join?token=<jwt>` | Upgrade to WebSocket and join the room |

### Health

```
GET /health  →  { "status": "ok", "rooms": <count> }
```

## WebSocket signaling protocol

Connect to `ws://localhost:4000/rooms/:id/join?token=<jwt>`.

### Messages sent by the server

| Type | Payload | Meaning |
|---|---|---|
| `joined` | `{ peer_id }` | You have joined; this is your assigned peer ID |
| `existing_peer` | `{ peer_id }` | A peer already in the room — they will initiate an offer to you |
| `peer_joined` | `{ peer_id }` | A new peer joined — you must send them an offer |
| `peer_left` | `{ peer_id }` | A peer disconnected |
| `offer_from` | `{ from_peer, sdp }` | Incoming WebRTC offer |
| `answer_from` | `{ from_peer, sdp }` | Incoming WebRTC answer |
| `ice_candidate_from` | `{ from_peer, candidate }` | Incoming ICE candidate |
| `chat_from` | `{ from_peer, text, timestamp_ms }` | A chat message from another peer; `timestamp_ms` is a Unix timestamp set by the server |
| `error` | `{ reason }` | Something went wrong |

### Messages sent by the client

| Type | Payload | Meaning |
|---|---|---|
| `leave` | — | Gracefully leave the room |
| `offer_to` | `{ target, sdp }` | Send a WebRTC offer to a peer |
| `answer_to` | `{ target, sdp }` | Send a WebRTC answer to a peer |
| `ice_candidate_to` | `{ target, candidate }` | Send an ICE candidate to a peer |
| `chat_message` | `{ text }` | Send a chat message; the server broadcasts it to all other peers in the room |

## Project structure

```
rundo/
├── crates/
│   ├── server/          # Axum HTTP + WebSocket server
│   │   ├── src/
│   │   │   ├── auth/    # JWT, Argon2, login/register handlers
│   │   │   ├── rooms/   # Room CRUD + registry
│   │   │   ├── signaling/  # WebSocket handler + peer session
│   │   │   ├── store/   # Database abstraction (SQLite / PostgreSQL / in-memory)
│   │   │   └── config.rs
│   │   └── migrations/
│   └── shared/          # Models (User, Room, PeerId) and message types
│       └── src/
│           ├── models.rs
│           └── messages.rs
└── client/              # Next.js frontend
    ├── app/
    │   ├── login/
    │   ├── register/
    │   └── rooms/
    │       ├── page.tsx         # Room list
    │       └── [id]/page.tsx    # Room view with video grid
    ├── components/
    │   ├── VideoGrid.tsx
    │   └── CreateRoomDialog.tsx
    ├── hooks/
    │   ├── useWebRTC.ts         # WebSocket + WebRTC logic
    │   └── useAuth.ts
    └── lib/
        └── api.ts               # Axios client
```

## Production checklist

- Set `JWT_SECRET` to a long random string
- Set `DATABASE_URL` to a PostgreSQL connection string
- Set `UDP_MEDIA_HOST` to the server's public IP or hostname so ICE candidates are reachable
- Place the frontend behind a TLS reverse proxy (browsers require HTTPS to access camera/microphone)
- Add a TURN server to `ICE_CONFIG` in `client/hooks/useWebRTC.ts` for peers behind strict NATs

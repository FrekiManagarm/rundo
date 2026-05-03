CREATE TABLE IF NOT EXISTS users (
    id          TEXT NOT NULL PRIMARY KEY,
    email       TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rooms (
    id          TEXT NOT NULL PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK(kind IN ('conference', 'stream')),
    owner_id    TEXT NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL
);

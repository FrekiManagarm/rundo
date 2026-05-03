use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use shared::models::{Room, RoomId, RoomKind, User, UserId};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::store::Store;

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Store for PostgresStore {
    async fn create_user(&self, user: User) -> Result<()> {
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id.0.to_string())
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .ok()??;
        row_to_user(&row)
    }

    async fn get_user_by_id(&self, id: UserId) -> Option<User> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .ok()??;
        row_to_user(&row)
    }

    async fn create_room(&self, room: Room) -> Result<()> {
        sqlx::query(
            "INSERT INTO rooms (id, name, kind, owner_id, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(room.id.0.to_string())
        .bind(&room.name)
        .bind(kind_to_str(&room.kind))
        .bind(room.owner_id.0.to_string())
        .bind(room.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_room(&self, id: RoomId) -> Option<Room> {
        let row = sqlx::query(
            "SELECT id, name, kind, owner_id, created_at FROM rooms WHERE id = $1",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .ok()??;
        row_to_room(&row)
    }

    async fn delete_room(&self, id: RoomId) -> Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = $1")
            .bind(id.0.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_rooms(&self) -> Vec<Room> {
        sqlx::query(
            "SELECT id, name, kind, owner_id, created_at FROM rooms ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(row_to_room)
        .collect()
    }
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Option<User> {
    let id: String = row.try_get("id").ok()?;
    let email: String = row.try_get("email").ok()?;
    let password_hash: String = row.try_get("password_hash").ok()?;
    let created_at: String = row.try_get("created_at").ok()?;
    Some(User {
        id: UserId(Uuid::parse_str(&id).ok()?),
        email,
        password_hash,
        created_at: DateTime::parse_from_rfc3339(&created_at).ok()?.to_utc(),
    })
}

fn row_to_room(row: &sqlx::postgres::PgRow) -> Option<Room> {
    let id: String = row.try_get("id").ok()?;
    let name: String = row.try_get("name").ok()?;
    let kind: String = row.try_get("kind").ok()?;
    let owner_id: String = row.try_get("owner_id").ok()?;
    let created_at: String = row.try_get("created_at").ok()?;
    Some(Room {
        id: RoomId(Uuid::parse_str(&id).ok()?),
        name,
        kind: str_to_kind(&kind)?,
        owner_id: UserId(Uuid::parse_str(&owner_id).ok()?),
        created_at: DateTime::parse_from_rfc3339(&created_at).ok()?.to_utc(),
    })
}

fn kind_to_str(kind: &RoomKind) -> &'static str {
    match kind {
        RoomKind::Conference => "conference",
        RoomKind::Stream => "stream",
    }
}

fn str_to_kind(s: &str) -> Option<RoomKind> {
    match s {
        "conference" => Some(RoomKind::Conference),
        "stream" => Some(RoomKind::Stream),
        _ => None,
    }
}

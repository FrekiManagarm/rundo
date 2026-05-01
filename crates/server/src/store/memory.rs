use anyhow::Result;
use dashmap::DashMap;
use shared::models::{Room, RoomId, User, UserId};
use crate::store::Store;

#[derive(Default)]
#[allow(dead_code)] // used in Task 5 when AppState is wired
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
        let id = *self.users_by_email.get(email)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::RoomKind;
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

    #[tokio::test]
    async fn get_user_by_id() {
        let store = InMemoryStore::default();
        let user = make_user();
        let id = user.id;
        store.create_user(user.clone()).await.unwrap();
        let found = store.get_user_by_id(id).await.unwrap();
        assert_eq!(found.email, user.email);
    }
}

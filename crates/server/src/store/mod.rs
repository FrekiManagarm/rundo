pub mod memory;

use anyhow::Result;
use async_trait::async_trait;
use shared::models::{Room, RoomId, User, UserId};

#[async_trait]
#[allow(dead_code)]
pub trait Store: Send + Sync {
    async fn create_user(&self, user: User) -> Result<()>;
    async fn get_user_by_email(&self, email: &str) -> Option<User>;
    async fn get_user_by_id(&self, id: UserId) -> Option<User>;
    async fn create_room(&self, room: Room) -> Result<()>;
    async fn get_room(&self, id: RoomId) -> Option<Room>;
    async fn delete_room(&self, id: RoomId) -> Result<()>;
    async fn list_rooms(&self) -> Vec<Room>;
}

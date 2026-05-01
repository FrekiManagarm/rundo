use shared::models::RoomId;
use tokio::sync::mpsc;

use crate::rooms::registry::RoomCommand;

pub async fn run_room(_room_id: RoomId, mut cmd_rx: mpsc::Receiver<RoomCommand>) {
    while let Some(_cmd) = cmd_rx.recv().await {
        // SFU fanout — implemented in Task 12
    }
}

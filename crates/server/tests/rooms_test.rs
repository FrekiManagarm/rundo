use axum_test::TestServer;
use serde_json::{json, Value};
use server::{build_router, config::Config, state::AppState, store::memory::InMemoryStore};

async fn test_server() -> TestServer {
    let config = Config::from_env();
    let state = AppState::new(config, InMemoryStore::default())
        .await
        .expect("AppState");
    TestServer::new(build_router(state))
}

async fn register_and_token(server: &TestServer) -> String {
    let resp = server
        .post("/auth/register")
        .json(&json!({"email": "test@rooms.com", "password": "pass123"}))
        .await;
    resp.json::<Value>()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn create_room_returns_201() {
    let server = test_server().await;
    let token = register_and_token(&server).await;
    let resp = server
        .post("/rooms")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({"name": "Test Room", "kind": "conference"}))
        .await;
    assert_eq!(resp.status_code(), 201);
    let body = resp.json::<Value>();
    assert_eq!(body["name"], "Test Room");
    assert_eq!(body["kind"], "conference");
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn list_rooms_returns_created_room() {
    let server = test_server().await;
    let token = register_and_token(&server).await;
    server
        .post("/rooms")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({"name": "My Room", "kind": "stream"}))
        .await;
    let resp = server
        .get("/rooms")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(resp.status_code(), 200);
    let rooms = resp.json::<Vec<Value>>();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0]["name"], "My Room");
}

#[tokio::test]
async fn get_room_returns_room() {
    let server = test_server().await;
    let token = register_and_token(&server).await;
    let create_resp = server
        .post("/rooms")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({"name": "Get Me", "kind": "conference"}))
        .await;
    let room_id = create_resp.json::<Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let resp = server
        .get(&format!("/rooms/{room_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(resp.status_code(), 200);
    assert_eq!(resp.json::<Value>()["name"], "Get Me");
}

#[tokio::test]
async fn delete_room_returns_204() {
    let server = test_server().await;
    let token = register_and_token(&server).await;
    let create_resp = server
        .post("/rooms")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({"name": "Delete Me", "kind": "conference"}))
        .await;
    let room_id = create_resp.json::<Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let del_resp = server
        .delete(&format!("/rooms/{room_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(del_resp.status_code(), 204);
    // Verify gone
    let get_resp = server
        .get(&format!("/rooms/{room_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(get_resp.status_code(), 404);
}

#[tokio::test]
async fn delete_room_by_non_owner_returns_403() {
    let server = test_server().await;
    // Owner creates room
    let owner_token = register_and_token(&server).await;
    let create_resp = server
        .post("/rooms")
        .add_header("Authorization", format!("Bearer {owner_token}"))
        .json(&json!({"name": "Protected", "kind": "conference"}))
        .await;
    let room_id = create_resp.json::<Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Different user tries to delete
    let other_resp = server
        .post("/auth/register")
        .json(&json!({"email": "other@rooms.com", "password": "pass123"}))
        .await;
    let other_token = other_resp.json::<Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();
    let del_resp = server
        .delete(&format!("/rooms/{room_id}"))
        .add_header("Authorization", format!("Bearer {other_token}"))
        .await;
    assert_eq!(del_resp.status_code(), 403);
}

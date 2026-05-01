use axum_test::TestServer;
use serde_json::{json, Value};
use server::{build_router, config::Config, state::AppState, store::memory::InMemoryStore};

async fn test_server() -> TestServer {
    let mut config = Config::from_env();
    config.udp_media_port = 0; // let the OS pick a free port
    let state = AppState::new(config, InMemoryStore::default())
        .await
        .expect("AppState");
    TestServer::new(build_router(state))
}

#[tokio::test]
async fn register_returns_201_with_token() {
    let server = test_server().await;
    let resp = server
        .post("/auth/register")
        .json(&json!({"email": "alice@test.com", "password": "pass123"}))
        .await;
    assert_eq!(resp.status_code(), 201);
    let body = resp.json::<Value>();
    assert!(body["token"].as_str().is_some());
    assert!(body["user_id"].as_str().is_some());
}

#[tokio::test]
async fn login_returns_200_with_token() {
    let server = test_server().await;
    server
        .post("/auth/register")
        .json(&json!({"email": "bob@test.com", "password": "pass123"}))
        .await;
    let resp = server
        .post("/auth/token")
        .json(&json!({"email": "bob@test.com", "password": "pass123"}))
        .await;
    assert_eq!(resp.status_code(), 200);
    let body = resp.json::<Value>();
    assert!(body["token"].as_str().is_some());
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let server = test_server().await;
    server
        .post("/auth/register")
        .json(&json!({"email": "carol@test.com", "password": "pass123"}))
        .await;
    let resp = server
        .post("/auth/token")
        .json(&json!({"email": "carol@test.com", "password": "wrong"}))
        .await;
    assert_eq!(resp.status_code(), 401);
}

#[tokio::test]
async fn duplicate_email_returns_409() {
    let server = test_server().await;
    server
        .post("/auth/register")
        .json(&json!({"email": "dave@test.com", "password": "pass123"}))
        .await;
    let resp = server
        .post("/auth/register")
        .json(&json!({"email": "dave@test.com", "password": "other"}))
        .await;
    assert_eq!(resp.status_code(), 409);
}

use axum_test::TestServer;
use serde_json::{json, Value};
use server::create_app;

#[tokio::test]
async fn register_returns_201_with_token() {
    let server = TestServer::new(create_app());
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
    let server = TestServer::new(create_app());
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
    let server = TestServer::new(create_app());
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
    let server = TestServer::new(create_app());
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

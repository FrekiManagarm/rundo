use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::models::{User, UserId};

use crate::{
    auth::{
        jwt::encode_jwt,
        password::{hash_password, verify_password},
    },
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: UserId,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    if state.store.get_user_by_email(&body.email).await.is_some() {
        return Err(AppError::Conflict("email already registered".to_string()));
    }
    let password_hash = hash_password(body.password).await.map_err(AppError::Internal)?;
    let user = User {
        id: UserId::new(),
        email: body.email,
        password_hash,
        created_at: Utc::now(),
    };
    let user_id = user.id;
    state.store.create_user(user).await.map_err(AppError::Internal)?;
    let token = encode_jwt(user_id, &state.config.jwt_secret).map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token, user_id })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let user = state
        .store
        .get_user_by_email(&body.email)
        .await
        .ok_or(AppError::Unauthorized)?;
    let valid = verify_password(body.password, user.password_hash)
        .await
        .map_err(AppError::Internal)?;
    if !valid {
        return Err(AppError::Unauthorized);
    }
    let token = encode_jwt(user.id, &state.config.jwt_secret).map_err(AppError::Internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
    }))
}

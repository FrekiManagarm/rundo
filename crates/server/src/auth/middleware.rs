// AuthUser is used by Axum's extractor system via FromRequestParts; suppress the false dead_code
// lint that fires before any handler actually extracts it.
#![allow(dead_code)]

use axum::{extract::FromRequestParts, http::request::Parts};
use shared::models::UserId;
use std::future::Future;

use crate::{auth::jwt::decode_jwt, error::AppError, state::AppState};

#[derive(Debug, Clone)]
pub struct AuthUser(pub UserId);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // 1. Try Bearer header
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
            // 2. Fall back to ?token= query param (for WebSocket upgrades)
            .or_else(|| {
                let query = parts.uri.query().unwrap_or("");
                query
                    .split('&')
                    .find(|s| s.starts_with("token="))
                    .and_then(|s| s.strip_prefix("token="))
                    .map(|s| s.to_string())
            });

        let jwt_secret = state.config.jwt_secret.clone();
        async move {
            let token = token.ok_or(AppError::Unauthorized)?;
            let claims =
                decode_jwt(&token, &jwt_secret).map_err(|_| AppError::Unauthorized)?;
            Ok(AuthUser(claims.sub))
        }
    }
}

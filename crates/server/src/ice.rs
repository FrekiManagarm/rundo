use anyhow::Result;
use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{auth::middleware::AuthUser, error::AppResult, state::AppState};

#[derive(Serialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

#[derive(Serialize)]
pub struct IceConfig {
    pub ice_servers: Vec<IceServer>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
) -> AppResult<Json<IceConfig>> {
    let mut servers = vec![IceServer {
        urls: state.config.stun_urls.clone(),
        username: None,
        credential: None,
    }];

    // Add TURN if configured — generate short-lived coturn credentials
    if !state.config.turn_urls.is_empty() {
        if let Some(secret) = &state.config.turn_secret {
            let expiry = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + state.config.turn_ttl_secs;
            // coturn `use-auth-secret` format: "{expiry}:{label}"
            let username = format!("{expiry}:rundo");
            let credential = hmac_sha1_b64(&username, secret)?;
            servers.push(IceServer {
                urls: state.config.turn_urls.clone(),
                username: Some(username),
                credential: Some(credential),
            });
        }
    }

    Ok(Json(IceConfig { ice_servers: servers }))
}

fn hmac_sha1_b64(data: &str, secret: &str) -> Result<String> {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("HMAC init: {e}"))?;
    mac.update(data.as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}

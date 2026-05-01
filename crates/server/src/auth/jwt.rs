use anyhow::{anyhow, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use shared::models::UserId;

#[allow(dead_code)]
const EXPIRY_SECS: u64 = 86_400; // 24 h

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Claims {
    pub sub: UserId,
    pub exp: u64,
}

#[allow(dead_code)]
pub fn encode_jwt(user_id: UserId, secret: &str) -> Result<String> {
    let exp = jsonwebtoken::get_current_timestamp() + EXPIRY_SECS;
    let claims = Claims { sub: user_id, exp };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("jwt encode: {e}"))
}

#[allow(dead_code)]
pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow!("jwt decode: {e}"))?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::UserId;

    const SECRET: &str = "test-secret";

    #[test]
    fn encode_and_decode_roundtrip() {
        let id = UserId::new();
        let token = encode_jwt(id, SECRET).unwrap();
        let claims = decode_jwt(&token, SECRET).unwrap();
        assert_eq!(claims.sub, id);
    }

    #[test]
    fn wrong_secret_fails_decode() {
        let token = encode_jwt(UserId::new(), SECRET).unwrap();
        assert!(decode_jwt(&token, "wrong-secret").is_err());
    }

    #[test]
    fn tampered_token_fails_decode() {
        let mut token = encode_jwt(UserId::new(), SECRET).unwrap();
        token.push('x');
        assert!(decode_jwt(&token, SECRET).is_err());
    }
}

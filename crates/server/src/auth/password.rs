use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

// OWASP minimum: m=19456 KiB (19 MB), t=2, p=1 — ~10x faster than Argon2::default()
fn argon2() -> Argon2<'static> {
    let params = Params::new(19456, 2, 1, None).expect("valid argon2 params");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

pub async fn hash_password(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || hash_sync(&password))
        .await
        .map_err(|e| anyhow::anyhow!("thread join: {e}"))?
}

pub async fn verify_password(password: String, hash: String) -> Result<bool> {
    tokio::task::spawn_blocking(move || verify_sync(&password, &hash))
        .await
        .map_err(|e| anyhow::anyhow!("thread join: {e}"))?
}

fn hash_sync(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    argon2()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hash error: {e}"))
}

fn verify_sync(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("parse hash: {e}"))?;
    Ok(argon2().verify_password(password.as_bytes(), &parsed).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_correct_password() {
        let hash = hash_sync("secret123").unwrap();
        assert!(verify_sync("secret123", &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_sync("secret123").unwrap();
        assert!(!verify_sync("wrong", &hash).unwrap());
    }

    #[test]
    fn same_password_produces_different_hashes() {
        let h1 = hash_sync("secret").unwrap();
        let h2 = hash_sync("secret").unwrap();
        assert_ne!(h1, h2);
    }
}

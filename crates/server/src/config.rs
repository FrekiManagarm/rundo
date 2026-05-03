#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub jwt_secret: String,
    pub database_url: String,
    /// Comma-separated STUN URLs (default: Google STUN).
    pub stun_urls: Vec<String>,
    /// Comma-separated TURN/TURNS URLs (empty = no TURN).
    pub turn_urls: Vec<String>,
    /// Shared secret for coturn `use-auth-secret` mode.
    pub turn_secret: Option<String>,
    /// Credential TTL in seconds (default 24 h).
    pub turn_ttl_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4000),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-prod".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://rundo.db".to_string()),
            stun_urls: std::env::var("STUN_URLS")
                .unwrap_or_else(|_| "stun:stun.l.google.com:19302".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            turn_urls: std::env::var("TURN_URLS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            turn_secret: std::env::var("TURN_SECRET").ok(),
            turn_ttl_secs: std::env::var("TURN_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(86400),
        }
    }
}

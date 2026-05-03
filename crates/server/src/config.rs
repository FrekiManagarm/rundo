#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub udp_media_port: u16,
    /// IP address advertised in SDP ICE candidates. Must be reachable by browsers.
    /// Defaults to 127.0.0.1 for local development; set UDP_MEDIA_HOST for LAN/production.
    pub udp_media_host: String,
    pub jwt_secret: String,
    /// `sqlite://rundo.db` for local dev, `postgres://user:pass@host/db` for production.
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4000),
            udp_media_port: std::env::var("UDP_MEDIA_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4001),
            udp_media_host: std::env::var("UDP_MEDIA_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-prod".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://rundo.db".to_string()),
        }
    }
}

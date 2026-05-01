// crates/server/src/config.rs
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub http_port: u16,
    pub udp_media_port: u16,
    pub jwt_secret: String,
}

impl Config {
    #[allow(dead_code)]
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
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-prod".to_string()),
        }
    }
}

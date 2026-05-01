use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();

    let config = server::config::Config::from_env();
    tracing::info!("Listening on 0.0.0.0:{}", config.http_port);
    let addr = format!("0.0.0.0:{}", config.http_port);
    let app = server::create_app();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

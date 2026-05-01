mod config;
mod error;

#[tokio::main]
async fn main() {
    let _config = config::Config::from_env();
    println!("WebRTC server starting on port {}…", _config.http_port);
}

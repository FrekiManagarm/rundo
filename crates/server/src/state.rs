use std::sync::Arc;

use crate::{config::Config, rooms::registry::RoomRegistry, sfu::SfuState, store::Store};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<dyn Store>,
    pub registry: Arc<RoomRegistry>,
    pub sfu: SfuState,
}

impl AppState {
    pub async fn new(config: Config, store: impl Store + 'static) -> anyhow::Result<Self> {
        let sfu_addr = format!("0.0.0.0:{}", config.udp_media_port);
        let sfu = SfuState::bind(&sfu_addr).await?;
        Ok(Self {
            config: Arc::new(config),
            store: Arc::new(store),
            registry: Arc::new(RoomRegistry::default()),
            sfu,
        })
    }
}

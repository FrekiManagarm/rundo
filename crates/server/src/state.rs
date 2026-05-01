use std::sync::Arc;

use crate::{config::Config, rooms::registry::RoomRegistry, store::Store};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<dyn Store>,
    pub registry: Arc<RoomRegistry>,
}

impl AppState {
    pub fn new(config: Config, store: impl Store + 'static) -> Self {
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
            registry: Arc::new(RoomRegistry::default()),
        }
    }
}

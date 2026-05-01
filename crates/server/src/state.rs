use std::sync::Arc;
use crate::config::Config;
use crate::store::memory::InMemoryStore;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<InMemoryStore>,
}

impl AppState {
    pub fn new(config: Config, store: InMemoryStore) -> Self {
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
        }
    }
}

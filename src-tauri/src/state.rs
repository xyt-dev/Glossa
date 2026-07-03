use std::path::PathBuf;

use kernel::client::Client;
use kernel::config::{self, Config};
use kernel::memory::MemoryStore;
use kernel::store::SessionStore;
use tokio::sync::Mutex;

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub client: Client,
}

impl AppState {
    pub fn init() -> kernel::Result<Self> {
        let config_path = config::default_config_path();
        let config = Config::load_or_init(&config_path)?;
        Ok(Self { config_path, config: Mutex::new(config), client: Client::new() })
    }

    pub fn memory_store(config: &Config) -> MemoryStore {
        MemoryStore::new(config.vocab_path())
    }

    pub fn session_store(config: &Config) -> SessionStore {
        SessionStore::new(config.sessions_dir())
    }
}

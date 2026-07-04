//! AppCore: UI 无关的应用门面。桌面（Tauri commands）与 Web（axum handlers）
//! 两个适配层都只做参数搬运，业务胶水统一放在这里。

use std::path::PathBuf;

use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

use crate::agent::{self, SendEvent};
use crate::client::Client;
use crate::config::{self, Config};
use crate::memory::{MarkInput, MarkKind, MemoryStore, VocabEntry, VocabMemory};
use crate::store::{Session, SessionMeta, SessionStore};
use crate::{Mode, Result};

pub struct AppCore {
    config_path: PathBuf,
    config: Mutex<Config>,
    client: Client,
}

impl AppCore {
    /// Load (or initialize) the config at the platform default path.
    pub fn init() -> Result<Self> {
        Self::init_with_path(config::default_config_path())
    }

    pub fn init_with_path(config_path: PathBuf) -> Result<Self> {
        let config = Config::load_or_init(&config_path)?;
        Ok(Self { config_path, config: Mutex::new(config), client: Client::new() })
    }

    pub async fn config(&self) -> Config {
        self.config.lock().await.clone()
    }

    /// Non-blocking snapshot for synchronous startup paths (e.g. Tauri setup).
    pub fn try_config(&self) -> Option<Config> {
        self.config.try_lock().ok().map(|c| c.clone())
    }

    pub async fn set_config(&self, config: Config) -> Result<()> {
        config.save(&self.config_path)?;
        *self.config.lock().await = config;
        Ok(())
    }

    fn memory_store(config: &Config) -> MemoryStore {
        MemoryStore::new(config.vocab_path())
    }

    fn session_store(config: &Config) -> SessionStore {
        SessionStore::new(config.sessions_dir())
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionMeta>> {
        Self::session_store(&self.config().await).list()
    }

    pub async fn create_session(&self) -> Result<Session> {
        Self::session_store(&self.config().await).create()
    }

    pub async fn load_session(&self, id: &str) -> Result<Session> {
        Self::session_store(&self.config().await).load(id)
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        Self::session_store(&self.config().await).delete(id)
    }

    pub async fn rename_session(&self, id: &str, title: &str) -> Result<Session> {
        Self::session_store(&self.config().await).rename(id, title)
    }

    /// One agent turn; the returned stream yields SendEvents until the turn
    /// is persisted (see agent::send).
    pub async fn send_message(
        &self,
        session_id: String,
        text: String,
        mode: Mode,
    ) -> Result<ReceiverStream<SendEvent>> {
        let config = self.config().await;
        let memory = Self::memory_store(&config);
        let store = Self::session_store(&config);
        agent::send(self.client.clone(), config, memory, store, session_id, text, mode).await
    }

    pub async fn mark(&self, input: MarkInput) -> Result<VocabEntry> {
        Self::memory_store(&self.config().await).mark(input)
    }

    pub async fn unmark(&self, word: &str, kind: MarkKind) -> Result<()> {
        Self::memory_store(&self.config().await).unmark(word, kind)
    }

    pub async fn memory(&self) -> Result<VocabMemory> {
        Self::memory_store(&self.config().await).load()
    }
}

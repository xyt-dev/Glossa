//! Translator kernel: UI-agnostic core (config, OpenAI-compatible client,
//! structured translation schema, vocab memory, session store, agent loop).

pub mod agent;
pub mod client;
pub mod config;
pub mod memory;
pub mod prompt;
pub mod schema;
pub mod store;

use serde::{Deserialize, Serialize};

/// Per-turn output mode: strict structured translation, or free-form chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Translate,
    Chat,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Translate
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error (status {status}): {body}")]
    Api { status: u16, body: String },
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml parse: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("config: {0}")]
    Config(String),
    #[error("profile '{0}' not found")]
    ProfileNotFound(String),
    #[error("no api key: set api_key in config.toml or export {0}")]
    MissingApiKey(String),
    #[error("session '{0}' not found")]
    SessionNotFound(String),
    #[error("stream: {0}")]
    Stream(String),
    #[error("model response has no content")]
    EmptyResponse,
}

pub type Result<T> = std::result::Result<T, Error>;

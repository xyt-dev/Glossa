use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Mode, Result};

/// Written verbatim on first run so the user gets a commented template.
pub const DEFAULT_CONFIG_TOML: &str = r#"# glossa 配置文件
# 任意 OpenAI 兼容 API 均可：填 base_url / model / api_key 即可。

active_profile = "deepseek"

[ui]
# gruvbox-light | gruvbox-dark | catppuccin-latte | catppuccin-mocha
theme = "gruvbox-light"
zoom = 1.0                            # 界面缩放（1.0 = 100%，建议 0.8-1.8）

[memory]
# 生词本路径，缺省为平台数据目录下 vocab.json
# path = "/path/to/vocab.json"
min_ielts_band = 7.0     # 模型选词讲解的 IELTS band 下限
max_context_words = 60   # 喂给模型的最近生词条数上限

[session]
default_mode = "translate"  # translate | chat
max_context_messages = 40   # 发给 API 的历史消息滑动窗口

[web]
enabled = false   # 桌面端启动时是否同时开启 Web 服务（`glossa web` 命令不受此影响）
port = 8040       # Web 服务端口（0.0.0.0，局域网可访问）

[[profiles]]
name = "deepseek"
base_url = "https://api.deepseek.com/v1"
api_key = ""                       # 直接填 key，或留空用下面的环境变量
api_key_env = "DEEPSEEK_API_KEY"
model = "deepseek-v4-pro-max"
# 思考强度（留空/删除 = no thinking）：low | medium | high | xhigh
# translate_effort = "high"           # 翻译模式（默认 no thinking）
chat_effort = "xhigh"                 # 聊天模式
# provider = "deepseek"               # 可选：thinking 字段兼容层（deepseek | openai），缺省按 base_url 自动判断
# temperature = 1.0                # 可选
# [profiles.extra]                 # 可选，任意额外请求字段原样透传
# top_p = 0.95
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub active_profile: String,
    pub ui: UiConfig,
    pub memory: MemoryConfig,
    pub session: SessionConfig,
    pub web: WebConfig,
    pub profiles: Vec<Profile>,
}

impl Default for Config {
    // NOTE: must not parse DEFAULT_CONFIG_TOML here — the container-level
    // #[serde(default)] calls Config::default() during deserialization,
    // which would recurse infinitely.
    fn default() -> Self {
        Self {
            active_profile: "deepseek".into(),
            ui: UiConfig::default(),
            memory: MemoryConfig::default(),
            session: SessionConfig::default(),
            web: WebConfig::default(),
            profiles: vec![Profile {
                name: "deepseek".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                api_key: String::new(),
                api_key_env: "DEEPSEEK_API_KEY".into(),
                model: "deepseek-v4-pro-max".into(),
                translate_effort: None,
                chat_effort: Some("xhigh".into()),
                ..Default::default()
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    /// Webview zoom factor (1.0 = 100%).
    pub zoom: f64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "gruvbox-light".into(),
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub min_ielts_band: f32,
    pub max_context_words: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            path: None,
            min_ielts_band: 7.0,
            max_context_words: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub default_mode: Mode,
    pub max_context_messages: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_mode: Mode::Translate,
            max_context_messages: 40,
        }
    }
}

/// 桌面端随启的内嵌 Web 服务。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    pub enabled: bool,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self { enabled: false, port: 8040 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_key_env: String,
    pub model: String,
    /// Per-mode reasoning effort: null → 不思考, "low"|"medium"|"high"|"xhigh" → enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translate_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_effort: Option<String>,
    /// Provider compat layer override ("deepseek" | "openai"). When absent the
    /// client sniffs the base_url; set it explicitly for aggregator URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
    /// Populated by agent.rs from translate_effort/chat_effort before calling the client.
    #[serde(skip)]
    pub effort: Option<String>,
}

impl Profile {
    pub fn resolve_api_key(&self) -> Result<String> {
        if !self.api_key.is_empty() {
            return Ok(self.api_key.clone());
        }
        if !self.api_key_env.is_empty() {
            if let Ok(v) = std::env::var(&self.api_key_env) {
                if !v.is_empty() {
                    return Ok(v);
                }
            }
        }
        Err(Error::MissingApiKey(self.api_key_env.clone()))
    }
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "glossa")
}

/// Platform config dir, e.g. `~/.config/glossa` on Linux.
pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Platform data dir, e.g. `~/.local/share/glossa` on Linux.
pub fn data_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// v0.1 profiles had a single `effort` field applied to every turn. Since the
/// per-mode split it would be silently dropped (it now only exists as a
/// #[serde(skip)] runtime field), losing the user's setting — map it to
/// `chat_effort` instead.
fn migrate_legacy(doc: &mut toml::Value) -> bool {
    let mut changed = false;
    if let Some(profiles) = doc.get_mut("profiles").and_then(toml::Value::as_array_mut) {
        for profile in profiles {
            if let Some(table) = profile.as_table_mut() {
                if let Some(effort) = table.remove("effort") {
                    changed = true;
                    if !table.contains_key("chat_effort") {
                        table.insert("chat_effort".into(), effort);
                    }
                }
            }
        }
    }
    changed
}

impl Config {
    /// Load from `path`, writing the commented default template first if absent.
    /// Legacy fields are migrated and, if any were found, written back.
    pub fn load_or_init(path: &Path) -> Result<Config> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, DEFAULT_CONFIG_TOML)?;
        }
        let mut doc: toml::Value = toml::from_str(&fs::read_to_string(path)?)?;
        let migrated = migrate_legacy(&mut doc);
        let cfg: Config = doc.try_into()?;
        if migrated {
            cfg.save(path)?;
        }
        Ok(cfg)
    }

    /// Serialize back to `path`. Comments from the template are not preserved.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, toml::to_string_pretty(self)?)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn active_profile(&self) -> Result<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.name == self.active_profile)
            .ok_or_else(|| Error::ProfileNotFound(self.active_profile.clone()))
    }

    pub fn vocab_path(&self) -> PathBuf {
        self.memory
            .path
            .clone()
            .unwrap_or_else(|| data_dir().join("vocab.json"))
    }

    pub fn sessions_dir(&self) -> PathBuf {
        data_dir().join("sessions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_template_parses() {
        let cfg: Config = toml::from_str(DEFAULT_CONFIG_TOML).unwrap();
        assert_eq!(cfg.active_profile, "deepseek");
        assert_eq!(cfg.profiles.len(), 1);
        let p = cfg.active_profile().unwrap();
        assert_eq!(p.model, "deepseek-v4-pro-max");
        assert_eq!(p.chat_effort.as_deref(), Some("xhigh"));
        assert!(
            p.translate_effort.is_none(),
            "translate should default to no thinking"
        );
        assert_eq!(cfg.session.default_mode, Mode::Translate);
        assert!(!cfg.web.enabled);
        assert_eq!(cfg.web.port, 8040);
        assert_eq!(cfg.ui.theme, "gruvbox-light");
        assert_eq!(cfg.ui.zoom, 1.0);
    }

    #[test]
    fn load_or_init_writes_template_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = Config::load_or_init(&path).unwrap();
        assert!(path.exists());
        // mutate + save + reload
        let mut cfg2 = cfg.clone();
        cfg2.ui.theme = "gruvbox-dark".into();
        cfg2.save(&path).unwrap();
        let cfg3 = Config::load_or_init(&path).unwrap();
        assert_eq!(cfg3.ui.theme, "gruvbox-dark");
    }

    #[test]
    fn legacy_effort_migrates_to_chat_effort() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"active_profile = "old"
[[profiles]]
name = "old"
base_url = "https://api.deepseek.com/v1"
model = "m"
effort = "high"
"#,
        )
        .unwrap();
        let cfg = Config::load_or_init(&path).unwrap();
        assert_eq!(cfg.profiles[0].chat_effort.as_deref(), Some("high"));
        assert!(cfg.profiles[0].translate_effort.is_none());
        // migrated config was written back without the legacy key
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("chat_effort"));
        assert!(!raw.contains("\neffort"));
        // an explicit chat_effort must not be overwritten by a leftover effort
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"x\"\neffort = \"low\"\nchat_effort = \"xhigh\"\n",
        )
        .unwrap();
        let cfg2 = Config::load_or_init(&path).unwrap();
        assert_eq!(cfg2.profiles[0].chat_effort.as_deref(), Some("xhigh"));
    }

    #[test]
    fn api_key_resolution_prefers_literal_then_env() {
        let mut p = Profile {
            api_key: "sk-lit".into(),
            api_key_env: "NOPE_UNSET".into(),
            ..Default::default()
        };
        assert_eq!(p.resolve_api_key().unwrap(), "sk-lit");
        p.api_key.clear();
        assert!(p.resolve_api_key().is_err());
    }
}

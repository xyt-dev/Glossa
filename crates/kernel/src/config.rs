use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Mode, Result};

/// Written verbatim on first run so the user gets a commented template.
pub const DEFAULT_CONFIG_TOML: &str = r#"# glossa 配置文件
# 任意 OpenAI 兼容 API 均可：填 base_url / model / api_key 即可。

active_profile = "deepseek"

[ui]
# gruvbox-dark | gruvbox-light | catppuccin-mocha | catppuccin-latte
theme = "catppuccin-mocha"
zoom = 1.2   # 界面缩放（1.0 = 100%，建议 0.8-1.8）

[memory]
# 生词本路径，缺省为平台数据目录下 vocab.json
# path = "/path/to/vocab.json"
min_ielts_band = 7.0     # 模型选词讲解的 IELTS band 下限
max_context_words = 60   # 喂给模型的最近生词条数上限

[session]
default_mode = "translate"  # translate | chat
max_context_messages = 40   # 发给 API 的历史消息滑动窗口

[[profiles]]
name = "deepseek"
base_url = "https://api.deepseek.com/v1"
api_key = ""                       # 直接填 key，或留空用下面的环境变量
api_key_env = "DEEPSEEK_API_KEY"
model = "deepseek-v4-pro-max"
effort = "high"                    # 可选，作为 reasoning_effort 传给 API
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
            profiles: vec![Profile {
                name: "deepseek".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                api_key: String::new(),
                api_key_env: "DEEPSEEK_API_KEY".into(),
                model: "deepseek-v4-pro-max".into(),
                effort: Some("high".into()),
                temperature: None,
                extra: None,
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
        Self { theme: "catppuccin-mocha".into(), zoom: 1.2 }
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
        Self { path: None, min_ielts_band: 7.0, max_context_words: 60 }
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
        Self { default_mode: Mode::Translate, max_context_messages: 40 }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Extra request-body fields passed through verbatim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
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
    project_dirs().map(|d| d.config_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

/// Platform data dir, e.g. `~/.local/share/glossa` on Linux.
pub fn data_dir() -> PathBuf {
    project_dirs().map(|d| d.data_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

impl Config {
    /// Load from `path`, writing the commented default template first if absent.
    pub fn load_or_init(path: &Path) -> Result<Config> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, DEFAULT_CONFIG_TOML)?;
        }
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
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
        self.memory.path.clone().unwrap_or_else(|| data_dir().join("vocab.json"))
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
        assert_eq!(p.effort.as_deref(), Some("high"));
        assert_eq!(cfg.session.default_mode, Mode::Translate);
        assert_eq!(cfg.ui.theme, "catppuccin-mocha");
        assert_eq!(cfg.ui.zoom, 1.2);
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
    fn api_key_resolution_prefers_literal_then_env() {
        let mut p = Profile { api_key: "sk-lit".into(), api_key_env: "NOPE_UNSET".into(), ..Default::default() };
        assert_eq!(p.resolve_api_key().unwrap(), "sk-lit");
        p.api_key.clear();
        assert!(p.resolve_api_key().is_err());
    }
}

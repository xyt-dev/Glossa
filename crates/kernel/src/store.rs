use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::schema::TranslationResult;
use crate::{Error, Mode, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub mode: Mode,
    /// User input, or assistant chat reply (markdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Parsed structured result of an assistant translate turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<TranslationResult>,
    /// Raw model output of an assistant translate turn (replayed as history;
    /// also the fallback display when parsing failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created: String,
    pub updated: String,
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created: String,
    pub updated: String,
}

impl Session {
    pub fn meta(&self) -> SessionMeta {
        SessionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            created: self.created.clone(),
            updated: self.updated.clone(),
        }
    }
}

pub const DEFAULT_TITLE: &str = "新会话";

#[derive(Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, id: &str) -> PathBuf {
        // ids are uuids we generate; sanitize anyway so a bad id can't escape the dir
        let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        self.dir.join(format!("{safe}.json"))
    }

    pub fn create(&self) -> Result<Session> {
        let now = chrono::Utc::now().to_rfc3339();
        let s = Session {
            id: uuid::Uuid::new_v4().to_string(),
            title: DEFAULT_TITLE.into(),
            created: now.clone(),
            updated: now,
            messages: vec![],
        };
        self.save(&s)?;
        Ok(s)
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let p = self.path(id);
        if !p.exists() {
            return Err(Error::SessionNotFound(id.into()));
        }
        Ok(serde_json::from_str(&fs::read_to_string(p)?)?)
    }

    pub fn save(&self, s: &Session) -> Result<()> {
        fs::create_dir_all(&self.dir)?;
        let p = self.path(&s.id);
        let tmp = p.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(s)?)?;
        fs::rename(&tmp, &p)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let p = self.path(id);
        if p.exists() {
            fs::remove_file(p)?;
        }
        Ok(())
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<Session> {
        let mut s = self.load(id)?;
        s.title = title.trim().to_string();
        // deliberately NOT bumping `updated`: renaming must not re-sort the
        // session list (list() orders by updated desc)
        self.save(&s)?;
        Ok(s)
    }

    /// All sessions, most recently updated first.
    pub fn list(&self) -> Result<Vec<SessionMeta>> {
        let mut out = vec![];
        if !self.dir.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            // skip files that fail to parse rather than breaking the whole list
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<Session>(&text) {
                    out.push(s.meta());
                }
            }
        }
        out.sort_by(|a, b| b.updated.cmp(&a.updated));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_list_load_delete_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions"));
        assert!(store.list().unwrap().is_empty());

        let a = store.create().unwrap();
        let mut b = store.create().unwrap();
        b.title = "第二个".into();
        b.updated = chrono::Utc::now().to_rfc3339();
        store.save(&b).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, b.id); // most recently updated first
        assert_eq!(list[0].title, "第二个");

        let loaded = store.load(&a.id).unwrap();
        assert_eq!(loaded.title, DEFAULT_TITLE);

        store.delete(&a.id).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(matches!(store.load(&a.id), Err(Error::SessionNotFound(_))));
    }

    #[test]
    fn rename_keeps_list_order() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions"));
        let a = store.create().unwrap();
        let mut b = store.create().unwrap();
        b.updated = chrono::Utc::now().to_rfc3339();
        store.save(&b).unwrap(); // b is most recent
        let renamed = store.rename(&a.id, "改名了").unwrap();
        assert_eq!(renamed.updated, a.updated); // timestamp untouched
        let list = store.list().unwrap();
        assert_eq!(list[0].id, b.id); // order unchanged, a did not jump to front
        assert_eq!(list[1].title, "改名了");
    }
}

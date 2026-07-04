use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkKind {
    /// 生词
    Word,
    /// native 用法
    Usage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VocabMemory {
    /// Model-maintained summary of the user's English level (reserved).
    pub profile_summary: String,
    pub words: Vec<VocabEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabEntry {
    pub word: String,
    pub kind: MarkKind,
    #[serde(default)]
    pub ipa: Option<String>,
    #[serde(default)]
    pub pos: Option<String>,
    #[serde(default)]
    pub meaning: Option<String>,
    #[serde(default)]
    pub native_usage: Option<String>,
    #[serde(default)]
    pub contexts: Vec<String>,
    pub first_marked: String,
    pub last_marked: String,
}

/// What the UI sends when the user marks a word / usage on a word card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkInput {
    pub word: String,
    pub kind: MarkKind,
    #[serde(default)]
    pub ipa: Option<String>,
    #[serde(default)]
    pub pos: Option<String>,
    #[serde(default)]
    pub meaning: Option<String>,
    #[serde(default)]
    pub native_usage: Option<String>,
    /// Sentence the word was encountered in.
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Clone)]
pub struct MemoryStore {
    path: PathBuf,
}

impl MemoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<VocabMemory> {
        if !self.path.exists() {
            return Ok(VocabMemory::default());
        }
        Ok(serde_json::from_str(&fs::read_to_string(&self.path)?)?)
    }

    fn save(&self, mem: &VocabMemory) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(mem)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Mark a word/usage: inserts once, then idempotently refreshes metadata.
    pub fn mark(&self, input: MarkInput) -> Result<VocabEntry> {
        let mut mem = self.load()?;
        let today = chrono::Utc::now().to_rfc3339();
        let existing = mem
            .words
            .iter_mut()
            .find(|e| e.kind == input.kind && e.word.eq_ignore_ascii_case(&input.word));
        let entry = match existing {
            Some(e) => {
                e.last_marked = today;
                if let Some(ctx) = input.context {
                    if !e.contexts.contains(&ctx) {
                        e.contexts.push(ctx);
                    }
                }
                e.ipa = input.ipa.or(e.ipa.take());
                e.pos = input.pos.or(e.pos.take());
                e.meaning = input.meaning.or(e.meaning.take());
                e.native_usage = input.native_usage.or(e.native_usage.take());
                e.clone()
            }
            None => {
                let e = VocabEntry {
                    word: input.word,
                    kind: input.kind,
                    ipa: input.ipa,
                    pos: input.pos,
                    meaning: input.meaning,
                    native_usage: input.native_usage,
                    contexts: input.context.into_iter().collect(),
                    first_marked: today.clone(),
                    last_marked: today,
                };
                mem.words.push(e.clone());
                e
            }
        };
        self.save(&mem)?;
        Ok(entry)
    }

    /// Remove a mark entirely (UI toggle-off).
    pub fn unmark(&self, word: &str, kind: MarkKind) -> Result<()> {
        let mut mem = self.load()?;
        mem.words.retain(|e| !(e.kind == kind && e.word.eq_ignore_ascii_case(word)));
        self.save(&mem)
    }

    /// Compact JSON context for the system prompt: the most recently marked
    /// entries (word/kind/meaning only), plus the profile summary.
    pub fn prompt_context(mem: &VocabMemory, max_words: usize) -> String {
        let mut entries: Vec<&VocabEntry> = mem.words.iter().collect();
        entries.sort_by(|a, b| b.last_marked.cmp(&a.last_marked));
        entries.truncate(max_words);
        let list: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "word": e.word,
                    "kind": e.kind,
                    "meaning": e.meaning,
                })
            })
            .collect();
        let mut out = String::new();
        if !mem.profile_summary.trim().is_empty() {
            out.push_str(&format!("水平画像：{}\n", mem.profile_summary.trim()));
        }
        if !list.is_empty() {
            out.push_str("最近标记记录：\n");
            out.push_str(&serde_json::to_string(&list).unwrap_or_default());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, MemoryStore) {
        let dir = tempfile::tempdir().unwrap();
        let s = MemoryStore::new(dir.path().join("vocab.json"));
        (dir, s)
    }

    fn input(word: &str) -> MarkInput {
        MarkInput {
            word: word.into(),
            kind: MarkKind::Word,
            ipa: Some("/x/".into()),
            pos: Some("n.".into()),
            meaning: Some("测试".into()),
            native_usage: None,
            context: Some("A test sentence.".into()),
        }
    }

    #[test]
    fn mark_is_idempotent_for_existing_word() {
        let (_d, s) = store();
        s.mark(input("ubiquitous")).unwrap();
        s.mark(input("Ubiquitous")).unwrap(); // case-insensitive merge
        assert_eq!(s.load().unwrap().words.len(), 1);
        s.unmark("ubiquitous", MarkKind::Word).unwrap();
        assert!(s.load().unwrap().words.is_empty());
    }

    #[test]
    fn prompt_context_truncates_to_most_recent() {
        let (_d, s) = store();
        for i in 0..5 {
            s.mark(input(&format!("w{i}"))).unwrap();
        }
        let mem = s.load().unwrap();
        let ctx = MemoryStore::prompt_context(&mem, 2);
        assert!(ctx.contains("w4") && ctx.contains("w3"));
        assert!(!ctx.contains("w0"));
    }

    #[test]
    fn empty_memory_yields_empty_context() {
        let ctx = MemoryStore::prompt_context(&VocabMemory::default(), 10);
        assert!(ctx.is_empty());
    }
}

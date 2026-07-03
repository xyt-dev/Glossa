use serde::{Deserialize, Serialize};

use crate::Result;

/// Structured result of a strict-translation turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translation: String,
    #[serde(default)]
    pub source_lang: Option<String>,
    #[serde(default)]
    pub target_lang: Option<String>,
    #[serde(default)]
    pub words: Vec<WordEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordEntry {
    pub word: String,
    #[serde(default)]
    pub ipa: Option<String>,
    #[serde(default)]
    pub pos: Option<String>,
    #[serde(default)]
    pub meaning: Option<String>,
    #[serde(default)]
    pub ielts_band: Option<f32>,
    #[serde(default)]
    pub native_usage: Option<String>,
    #[serde(default)]
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    pub en: String,
    pub zh: String,
}

/// Schema description embedded in the system prompt (kept human-readable
/// rather than formal JSON Schema — models follow it better).
pub const SCHEMA_TEXT: &str = r#"{
  "translation": "严格对应的译文（string，必填）",
  "source_lang": "原文语言代码，如 en / zh",
  "target_lang": "译文语言代码",
  "words": [
    {
      "word": "选中的词或短语",
      "ipa": "IPA 音标，如 /juːˈbɪkwɪtəs/（非英语词可省略）",
      "pos": "词性缩写，如 adj. / n. / v. / phr.",
      "meaning": "该词在本句语境中的中文释义",
      "ielts_band": 8,
      "native_usage": "native speaker 的典型用法/搭配/语感说明（中文讲解，可含英文搭配）",
      "examples": [ { "en": "英文例句", "zh": "例句中文翻译" } ]
    }
  ]
}"#;

/// Best-effort extraction of the JSON object from raw model output:
/// strips markdown fences and any prose around the outermost `{...}`.
fn extract_json(raw: &str) -> &str {
    let s = raw.trim();
    match (s.find('{'), s.rfind('}')) {
        (Some(a), Some(b)) if b >= a => &s[a..=b],
        _ => s,
    }
}

pub fn parse_translation(raw: &str) -> Result<TranslationResult> {
    Ok(serde_json::from_str(extract_json(raw))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"translation":"你好","source_lang":"en","target_lang":"zh","words":[{"word":"ubiquitous","ipa":"/juːˈbɪkwɪtəs/","pos":"adj.","meaning":"无处不在的","ielts_band":8,"native_usage":"常与 become 连用","examples":[{"en":"Smartphones are ubiquitous.","zh":"智能手机无处不在。"}]}]}"#;

    #[test]
    fn parses_plain_json() {
        let r = parse_translation(GOOD).unwrap();
        assert_eq!(r.translation, "你好");
        assert_eq!(r.words.len(), 1);
        assert_eq!(r.words[0].examples[0].zh, "智能手机无处不在。");
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let wrapped = format!("好的，以下是结果：\n```json\n{GOOD}\n```\n希望有帮助。");
        let r = parse_translation(&wrapped).unwrap();
        assert_eq!(r.translation, "你好");
    }

    #[test]
    fn minimal_object_fills_defaults() {
        let r = parse_translation(r#"{"translation":"hi"}"#).unwrap();
        assert!(r.words.is_empty());
        assert!(r.source_lang.is_none());
    }

    #[test]
    fn garbage_fails() {
        assert!(parse_translation("I cannot do that.").is_err());
    }
}

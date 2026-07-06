use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Error, Result};

/// Structured result of a strict-translation turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationResult {
    /// 完整整体译文。
    #[serde(default)]
    pub translation: String,
    /// 逐句对照：长段落按句切分，逐句给出译文。
    #[serde(default)]
    pub sentences: Vec<SentencePair>,
    #[serde(default)]
    pub source_lang: Option<String>,
    #[serde(default)]
    pub target_lang: Option<String>,
    /// 值得学习的词汇卡。
    #[serde(default)]
    pub words: Vec<WordEntry>,
    /// 原句中出现的 native 表达卡（独立于 words）。
    #[serde(default)]
    pub usages: Vec<UsageEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentencePair {
    pub src: String,
    pub dst: String,
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
    /// v0.2 schema had per-word native notes; kept only so old sessions still
    /// render. No longer part of the prompt schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_usage: Option<String>,
    #[serde(default)]
    pub examples: Vec<Example>,
}

/// A noteworthy native expression found in the source sentence
/// (idiom / collocation / sentence pattern), explained as its own card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEntry {
    /// The expression, quoted verbatim from the source.
    pub usage: String,
    #[serde(default)]
    pub explanation: Option<String>,
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
  "translation": "完整整体译文（string，必填）",
  "sentences": [
    { "src": "原文中的一个句子", "dst": "该句的对应译文" }
  ],
  "source_lang": "原文语言代码，如 en / zh",
  "target_lang": "译文语言代码",
  "words": [
    {
      "word": "选中的词或短语",
      "ipa": "IPA 音标，如 /juːˈbɪkwɪtəs/（非英语词可省略）",
      "pos": "词性缩写，如 adj. / n. / v. / phr.",
      "meaning": "该词在本句语境中的中文释义",
      "ielts_band": 8,
      "examples": [ { "en": "英文例句", "zh": "例句中文翻译" } ]
    }
  ],
  "usages": [
    {
      "usage": "原句中出现的 native 表达（习语/固定搭配/句式，原样摘出）",
      "explanation": "中文讲解：什么场合用、语感、常见搭配",
      "examples": [ { "en": "英文例句", "zh": "例句中文翻译" } ]
    }
  ]
}"#;

/// JSON Schema used for Anthropic-style structured outputs.
pub fn output_json_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "translation": { "type": "string" },
            "sentences": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "src": { "type": "string" },
                        "dst": { "type": "string" }
                    },
                    "required": ["src", "dst"],
                    "additionalProperties": false
                }
            },
            "source_lang": { "type": ["string", "null"] },
            "target_lang": { "type": ["string", "null"] },
            "words": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "word": { "type": "string" },
                        "ipa": { "type": ["string", "null"] },
                        "pos": { "type": ["string", "null"] },
                        "meaning": { "type": ["string", "null"] },
                        "ielts_band": { "type": ["number", "null"] },
                        "examples": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "en": { "type": "string" },
                                    "zh": { "type": "string" }
                                },
                                "required": ["en", "zh"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["word", "examples"],
                    "additionalProperties": false
                }
            },
            "usages": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "usage": { "type": "string" },
                        "explanation": { "type": ["string", "null"] },
                        "examples": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "en": { "type": "string" },
                                    "zh": { "type": "string" }
                                },
                                "required": ["en", "zh"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["usage", "examples"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["translation", "sentences", "words", "usages"],
        "additionalProperties": false
    })
}

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
    let r: TranslationResult = serde_json::from_str(extract_json(raw))?;
    // A result carrying neither a translation nor sentence pairs is useless —
    // treat it as unparseable so the repair/fallback path kicks in.
    if r.translation.trim().is_empty() && r.sentences.is_empty() {
        return Err(Error::EmptyResponse);
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"translation":"你好","source_lang":"en","target_lang":"zh","words":[{"word":"ubiquitous","ipa":"/juːˈbɪkwɪtəs/","pos":"adj.","meaning":"无处不在的","ielts_band":8,"native_usage":"常与 become 连用","examples":[{"en":"Smartphones are ubiquitous.","zh":"智能手机无处不在。"}]}]}"#;

    #[test]
    fn parses_plain_json_including_legacy_native_usage() {
        let r = parse_translation(GOOD).unwrap();
        assert_eq!(r.translation, "你好");
        assert_eq!(r.words.len(), 1);
        assert_eq!(r.words[0].native_usage.as_deref(), Some("常与 become 连用"));
        assert!(r.sentences.is_empty());
        assert!(r.usages.is_empty());
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let wrapped = format!("好的，以下是结果：\n```json\n{GOOD}\n```\n希望有帮助。");
        let r = parse_translation(&wrapped).unwrap();
        assert_eq!(r.translation, "你好");
    }

    #[test]
    fn parses_sentences_and_usages() {
        let raw = r#"{
          "translation": "整体译文。",
          "sentences": [
            {"src": "First sentence.", "dst": "第一句。"},
            {"src": "Second sentence.", "dst": "第二句。"}
          ],
          "usages": [
            {"usage": "as far as ... is concerned", "explanation": "就……而言",
             "examples": [{"en": "As far as I'm concerned, it works.", "zh": "就我而言，它可行。"}]}
          ]
        }"#;
        let r = parse_translation(raw).unwrap();
        assert_eq!(r.sentences.len(), 2);
        assert_eq!(r.sentences[1].dst, "第二句。");
        assert_eq!(r.usages[0].usage, "as far as ... is concerned");
        assert_eq!(r.usages[0].examples.len(), 1);
    }

    #[test]
    fn sentences_alone_is_valid() {
        let r = parse_translation(r#"{"sentences":[{"src":"Hi.","dst":"你好。"}]}"#).unwrap();
        assert_eq!(r.sentences.len(), 1);
    }

    #[test]
    fn minimal_object_fills_defaults() {
        let r = parse_translation(r#"{"translation":"hi"}"#).unwrap();
        assert!(r.words.is_empty());
        assert!(r.source_lang.is_none());
    }

    #[test]
    fn output_schema_is_strict_object() {
        let s = output_json_schema();
        assert_eq!(s["type"], "object");
        assert_eq!(s["additionalProperties"], false);
        assert!(s["properties"]["words"].is_object());
    }

    #[test]
    fn empty_result_is_rejected() {
        assert!(parse_translation(r#"{"words":[]}"#).is_err());
    }

    #[test]
    fn garbage_fails() {
        assert!(parse_translation("I cannot do that.").is_err());
    }
}

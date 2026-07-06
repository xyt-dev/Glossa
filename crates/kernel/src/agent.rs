use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::client::{ChatMessage, Client, Delta};
use crate::config::{Config, Profile};
use crate::memory::MemoryStore;
use crate::prompt;
use crate::schema::{self, TranslationResult};
use crate::store::{Message, Role, SessionMeta, SessionStore, DEFAULT_TITLE};
use crate::{Mode, Result};

/// Events streamed to the frontend during one turn.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SendEvent {
    /// Reasoning ("thinking") token delta, shown before the answer.
    Reasoning {
        text: String,
    },
    /// Raw token delta (typewriter display).
    Delta {
        text: String,
    },
    /// Translate turn parsed successfully.
    Parsed {
        result: TranslationResult,
    },
    /// Translate turn could not be parsed even after repair; show raw text.
    Fallback {
        raw: String,
    },
    /// Turn persisted; session meta may carry a new auto-generated title.
    Done {
        session: SessionMeta,
    },
    Error {
        message: String,
    },
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// A translate reply replayed for chat context = the full raw JSON (so chat can
/// reference word cards). For translate context we only need the plain
/// translation text (term/style consistency), not the card JSON.
fn translate_reply_text(m: &Message) -> Option<String> {
    if let Some(r) = &m.result {
        if !r.translation.trim().is_empty() {
            return Some(r.translation.clone());
        }
        if !r.sentences.is_empty() {
            return Some(
                r.sentences
                    .iter()
                    .map(|s| s.dst.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            );
        }
    }
    m.raw.clone()
}

/// Replay persisted history as API messages, filtered by the CURRENT turn mode:
///
/// - **Translate**: only past *translate* turns, and assistant replies as plain
///   translation text (no chat noise, no card JSON) — keeps term/style
///   consistency without bloating tokens or leaking chat into translation.
/// - **Chat**: the full history — translate replies as raw JSON so chat can
///   reference earlier translations and word cards.
fn history_to_api(messages: &[Message], window: usize, current: Mode) -> Vec<ChatMessage> {
    let kept: Vec<&Message> = match current {
        Mode::Translate => messages.iter().filter(|m| m.mode == Mode::Translate).collect(),
        Mode::Chat => messages.iter().collect(),
    };
    let start = kept.len().saturating_sub(window);
    kept[start..]
        .iter()
        .filter_map(|m| match m.role {
            Role::User => m
                .text
                .as_ref()
                .map(|t| ChatMessage::user(prompt::tag_user_text(m.mode, t))),
            Role::Assistant => match (current, m.mode) {
                // translate context: assistant → plain translation only
                (Mode::Translate, Mode::Translate) => {
                    translate_reply_text(m).map(ChatMessage::assistant)
                }
                // chat context: translate replies → full JSON (referenceable)
                (Mode::Chat, Mode::Translate) => m
                    .raw
                    .clone()
                    .or_else(|| {
                        m.result
                            .as_ref()
                            .and_then(|r| serde_json::to_string(r).ok())
                    })
                    .map(ChatMessage::assistant),
                (_, Mode::Chat) => m.text.clone().map(ChatMessage::assistant),
            },
        })
        .collect()
}

/// One agent turn: build context, stream the completion, enforce the output
/// contract (parse → repair → fallback for translate turns), persist, notify.
pub async fn send(
    client: Client,
    config: Config,
    memory: MemoryStore,
    store: SessionStore,
    session_id: String,
    text: String,
    mode: Mode,
) -> Result<ReceiverStream<SendEvent>> {
    let mut session = store.load(&session_id)?;
    let mut profile = config.active_profile()?.clone();
    // Collapse per-mode effort into a single request field for client.rs.
    profile.effort = match mode {
        Mode::Translate => profile.translate_effort.take(),
        Mode::Chat => profile.chat_effort.take(),
    };
    let mem = memory.load()?;
    let mem_ctx = MemoryStore::prompt_context(&mem, config.memory.max_context_words);

    // Per-mode system prompt + history window: translate and chat are isolated
    // (see prompt.rs / history_to_api).
    let (system, window) = match mode {
        Mode::Translate => (
            prompt::translate_system_prompt(&mem_ctx, config.memory.min_ielts_band),
            config.session.translate_context_messages,
        ),
        Mode::Chat => (
            prompt::chat_system_prompt(&mem_ctx),
            config.session.max_context_messages,
        ),
    };
    let mut api_messages = vec![ChatMessage::system(system)];
    api_messages.extend(history_to_api(&session.messages, window, mode));
    api_messages.push(ChatMessage::user(prompt::tag_user_text(mode, &text)));

    // Persist the user turn immediately so it survives a failed request.
    if session.messages.is_empty() && session.title == DEFAULT_TITLE {
        let title: String = text.chars().take(24).collect::<String>().trim().to_string();
        if !title.is_empty() {
            session.title = title;
        }
    }
    session.messages.push(Message {
        role: Role::User,
        mode,
        text: Some(text),
        result: None,
        raw: None,
        reasoning: None,
        ts: now(),
    });
    session.updated = now();
    store.save(&session)?;

    let (tx, rx) = mpsc::channel::<SendEvent>(64);
    tokio::spawn(async move {
        match run_turn(&client, &profile, api_messages, mode, &tx).await {
            Ok((outcome, reasoning)) => {
                // Keep the thinking trace only if the model actually produced one.
                let reasoning = Some(reasoning).filter(|r| !r.trim().is_empty());
                let msg = match outcome {
                    Outcome::Chat(text) => Message {
                        role: Role::Assistant,
                        mode,
                        text: Some(text),
                        result: None,
                        raw: None,
                        reasoning,
                        ts: now(),
                    },
                    Outcome::Translated { result, raw } => Message {
                        role: Role::Assistant,
                        mode,
                        text: None,
                        result: Some(result),
                        raw: Some(raw),
                        reasoning,
                        ts: now(),
                    },
                    Outcome::Fallback(raw) => Message {
                        role: Role::Assistant,
                        mode,
                        text: None,
                        result: None,
                        raw: Some(raw),
                        reasoning,
                        ts: now(),
                    },
                };
                session.messages.push(msg);
                session.updated = now();
                if let Err(e) = store.save(&session) {
                    let _ = tx
                        .send(SendEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    return;
                }
                let _ = tx
                    .send(SendEvent::Done {
                        session: session.meta(),
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(SendEvent::Error {
                        message: e.to_string(),
                    })
                    .await;
            }
        }
    });
    Ok(ReceiverStream::new(rx))
}

enum Outcome {
    Chat(String),
    Translated {
        result: TranslationResult,
        raw: String,
    },
    Fallback(String),
}

/// Returns the resolved outcome plus the accumulated reasoning ("thinking")
/// trace. Reasoning is streamed and collected separately from `full` so it
/// never pollutes the answer that gets parsed / persisted.
async fn run_turn(
    client: &Client,
    profile: &Profile,
    messages: Vec<ChatMessage>,
    mode: Mode,
    tx: &mpsc::Sender<SendEvent>,
) -> Result<(Outcome, String)> {
    let json_mode = mode == Mode::Translate;
    let mut stream = client.chat_stream(profile, &messages, json_mode).await?;
    let mut full = String::new();
    let mut reasoning = String::new();
    while let Some(item) = stream.next().await {
        // A dropped receiver just means the UI stopped listening; keep going
        // so the turn is still persisted.
        match item? {
            Delta::Content(delta) => {
                full.push_str(&delta);
                let _ = tx.send(SendEvent::Delta { text: delta }).await;
            }
            Delta::Reasoning(delta) => {
                reasoning.push_str(&delta);
                let _ = tx.send(SendEvent::Reasoning { text: delta }).await;
            }
        }
    }
    let outcome = match mode {
        Mode::Chat => Outcome::Chat(full),
        Mode::Translate => match schema::parse_translation(&full) {
            Ok(result) => {
                let _ = tx
                    .send(SendEvent::Parsed {
                        result: result.clone(),
                    })
                    .await;
                Outcome::Translated { result, raw: full }
            }
            Err(err) => {
                let (sys, usr) = prompt::repair_messages(&full, &err.to_string());
                let repaired = client
                    .chat_once(
                        profile,
                        &[ChatMessage::system(sys), ChatMessage::user(usr)],
                        true,
                    )
                    .await
                    .and_then(|raw| schema::parse_translation(&raw).map(|r| (r, raw)));
                match repaired {
                    Ok((result, raw)) => {
                        let _ = tx
                            .send(SendEvent::Parsed {
                                result: result.clone(),
                            })
                            .await;
                        Outcome::Translated { result, raw }
                    }
                    Err(_) => {
                        let _ = tx.send(SendEvent::Fallback { raw: full.clone() }).await;
                        Outcome::Fallback(full)
                    }
                }
            }
        },
    };
    Ok((outcome, reasoning))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Session;
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sse_body(chunks: &[&str]) -> String {
        let mut out = String::new();
        for c in chunks {
            let payload = json!({"choices":[{"delta":{"content": c}}]});
            out.push_str(&format!("data: {payload}\n\n"));
        }
        out.push_str("data: [DONE]\n\n");
        out
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        config: Config,
        memory: MemoryStore,
        store: SessionStore,
        session: Session,
    }

    async fn fixture(server: &MockServer) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.profiles[0].base_url = server.uri();
        config.profiles[0].api_key = "test-key".into();
        let memory = MemoryStore::new(dir.path().join("vocab.json"));
        let store = SessionStore::new(dir.path().join("sessions"));
        let session = store.create().unwrap();
        Fixture {
            _dir: dir,
            config,
            memory,
            store,
            session,
        }
    }

    async fn collect(f: &Fixture, text: &str, mode: Mode) -> Vec<SendEvent> {
        let stream = send(
            Client::direct(),
            f.config.clone(),
            f.memory.clone(),
            f.store.clone(),
            f.session.id.clone(),
            text.into(),
            mode,
        )
        .await
        .unwrap();
        stream.collect::<Vec<_>>().await
    }

    #[tokio::test]
    async fn translate_turn_streams_parses_and_persists() {
        let server = MockServer::start().await;
        let good = r#"{"translation":"智能手机无处不在。","words":[{"word":"ubiquitous","ipa":"/juːˈbɪkwɪtəs/","pos":"adj.","meaning":"无处不在的","ielts_band":8,"examples":[{"en":"E","zh":"译"}]}]}"#;
        // split raw JSON across two SSE chunks (16 is a char boundary)
        let (a, b) = good.split_at(16);
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(
                json!({"stream": true, "response_format": {"type": "json_object"}}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body(&[a, b]), "text/event-stream"),
            )
            .mount(&server)
            .await;

        let f = fixture(&server).await;
        let events = collect(&f, "Smartphones are ubiquitous.", Mode::Translate).await;

        assert!(matches!(events[0], SendEvent::Delta { .. }));
        assert!(events.iter().any(|e| matches!(e, SendEvent::Parsed { result } if result.translation == "智能手机无处不在。")));
        let done = events.iter().find_map(|e| match e {
            SendEvent::Done { session } => Some(session.clone()),
            _ => None,
        });
        // auto title from first message
        assert!(done.unwrap().title.starts_with("Smartphones"));

        let saved = f.store.load(&f.session.id).unwrap();
        assert_eq!(saved.messages.len(), 2);
        assert_eq!(
            saved.messages[1].result.as_ref().unwrap().words[0].word,
            "ubiquitous"
        );
        assert!(saved.messages[1].raw.is_some());
    }

    #[tokio::test]
    async fn broken_translate_output_is_repaired() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": true})))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                sse_body(&["{\"translation\": \"你好\"", " <-- truncated garbage"]),
                "text/event-stream",
            ))
            .mount(&server)
            .await;
        // repair call is non-streaming
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": false})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices":[{"message":{"role":"assistant","content":"{\"translation\":\"你好\",\"words\":[]}"}}]
            })))
            .mount(&server)
            .await;

        let f = fixture(&server).await;
        let events = collect(&f, "hello", Mode::Translate).await;
        assert!(events
            .iter()
            .any(|e| matches!(e, SendEvent::Parsed { result } if result.translation == "你好")));

        let saved = f.store.load(&f.session.id).unwrap();
        assert_eq!(
            saved.messages[1].result.as_ref().unwrap().translation,
            "你好"
        );
    }

    #[tokio::test]
    async fn unrepairable_translate_falls_back_to_raw() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": true})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(sse_body(&["not json at all"]), "text/event-stream"),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": false})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices":[{"message":{"role":"assistant","content":"still not json"}}]
            })))
            .mount(&server)
            .await;

        let f = fixture(&server).await;
        let events = collect(&f, "hello", Mode::Translate).await;
        assert!(events
            .iter()
            .any(|e| matches!(e, SendEvent::Fallback { raw } if raw.contains("not json"))));
        let saved = f.store.load(&f.session.id).unwrap();
        assert!(saved.messages[1].result.is_none());
        assert!(saved.messages[1].raw.is_some());
    }

    #[tokio::test]
    async fn chat_turn_streams_markdown_and_replays_history() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": true})))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                sse_body(&["**ubiquitous** 的意思是", "无处不在"]),
                "text/event-stream",
            ))
            .mount(&server)
            .await;

        let f = fixture(&server).await;
        // seed history with a prior translate turn
        let mut s = f.store.load(&f.session.id).unwrap();
        s.messages.push(Message {
            role: Role::User,
            mode: Mode::Translate,
            text: Some("Smartphones are ubiquitous.".into()),
            result: None,
            raw: None,
            reasoning: None,
            ts: now(),
        });
        s.messages.push(Message {
            role: Role::Assistant,
            mode: Mode::Translate,
            text: None,
            result: None,
            raw: Some("{\"translation\":\"智能手机无处不在。\"}".into()),
            reasoning: None,
            ts: now(),
        });
        f.store.save(&s).unwrap();

        let events = collect(&f, "为什么用 ubiquitous？", Mode::Chat).await;
        let deltas: String = events
            .iter()
            .filter_map(|e| match e {
                SendEvent::Delta { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, "**ubiquitous** 的意思是无处不在");

        let saved = f.store.load(&f.session.id).unwrap();
        assert_eq!(saved.messages.len(), 4);
        assert_eq!(
            saved.messages[3].text.as_deref(),
            Some("**ubiquitous** 的意思是无处不在")
        );

        // chat context: full history, translate reply as raw JSON
        let api = history_to_api(&saved.messages, 40, Mode::Chat);
        assert!(api[0].content.starts_with("[translate]"));
        assert!(api[1].content.contains("智能手机"));
        assert!(api[2].content.starts_with("[chat]"));
    }

    #[tokio::test]
    async fn reasoning_is_streamed_and_persisted_apart_from_answer() {
        let server = MockServer::start().await;
        // First reasoning_content deltas (thinking), then content deltas (answer).
        let mut body = String::new();
        for r in ["让我想想", "……"] {
            let p = json!({"choices":[{"delta":{"reasoning_content": r}}]});
            body.push_str(&format!("data: {p}\n\n"));
        }
        for c in ["答案", "在此"] {
            let p = json!({"choices":[{"delta":{"content": c}}]});
            body.push_str(&format!("data: {p}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({"stream": true})))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .mount(&server)
            .await;

        let f = fixture(&server).await;
        let events = collect(&f, "问题", Mode::Chat).await;

        let reasoning: String = events
            .iter()
            .filter_map(|e| match e {
                SendEvent::Reasoning { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        let content: String = events
            .iter()
            .filter_map(|e| match e {
                SendEvent::Delta { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        // reasoning surfaced on its own channel; answer deltas exclude it
        assert_eq!(reasoning, "让我想想……");
        assert_eq!(content, "答案在此");

        let saved = f.store.load(&f.session.id).unwrap();
        assert_eq!(saved.messages[1].text.as_deref(), Some("答案在此"));
        assert_eq!(saved.messages[1].reasoning.as_deref(), Some("让我想想……"));
    }

    #[test]
    fn history_window_truncates() {
        let mk = |i: usize| Message {
            role: Role::User,
            mode: Mode::Chat,
            text: Some(format!("m{i}")),
            result: None,
            raw: None,
            reasoning: None,
            ts: now(),
        };
        let msgs: Vec<Message> = (0..10).map(mk).collect();
        let api = history_to_api(&msgs, 3, Mode::Chat);
        assert_eq!(api.len(), 3);
        assert!(api[0].content.ends_with("m7"));
        assert!(api[2].content.ends_with("m9"));
    }

    #[test]
    fn translate_context_excludes_chat_and_card_json() {
        let ts = now();
        let user = |mode, t: &str| Message {
            role: Role::User,
            mode,
            text: Some(t.into()),
            result: None,
            raw: None,
            reasoning: None,
            ts: ts.clone(),
        };
        let translate_reply = Message {
            role: Role::Assistant,
            mode: Mode::Translate,
            text: None,
            result: Some(TranslationResult {
                translation: "智能手机无处不在。".into(),
                sentences: vec![],
                source_lang: None,
                target_lang: None,
                words: vec![],
                usages: vec![],
            }),
            raw: Some("{\"translation\":\"智能手机无处不在。\",\"words\":[/*卡片JSON*/]}".into()),
            reasoning: None,
            ts: ts.clone(),
        };
        let chat_reply = Message {
            role: Role::Assistant,
            mode: Mode::Chat,
            text: Some("这是聊天讲解噪音".into()),
            result: None,
            raw: None,
            reasoning: None,
            ts: ts.clone(),
        };
        let msgs = vec![
            user(Mode::Translate, "Smartphones are ubiquitous."),
            translate_reply,
            user(Mode::Chat, "为什么用 ubiquitous？"),
            chat_reply,
        ];

        // 翻译上下文：只保留翻译回合，assistant 只回放译文（不含卡片 JSON），聊天被过滤
        let api = history_to_api(&msgs, 12, Mode::Translate);
        assert_eq!(api.len(), 2, "只应剩翻译回合的 user + assistant");
        assert!(api[0].content.starts_with("[translate]"));
        assert_eq!(api[1].content, "智能手机无处不在。"); // 纯译文，不是 JSON
        assert!(!api[1].content.contains("words"), "翻译上下文不应含卡片 JSON");
        assert!(
            !api.iter().any(|m| m.content.contains("聊天讲解噪音")),
            "翻译上下文不应含聊天内容"
        );
    }
}

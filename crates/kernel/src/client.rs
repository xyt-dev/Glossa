use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::config::Profile;
use crate::schema;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A streamed chunk from the model: either reasoning ("thinking") or the
/// user-visible answer content. Providers expose different wire formats, but
/// the rest of the app only consumes this semantic split.
#[derive(Debug, Clone)]
pub enum Delta {
    Reasoning(String),
    Content(String),
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderKind {
    OpenAi,
    DeepSeek,
    Anthropic,
}

impl ProviderKind {
    fn from_profile(profile: &Profile) -> Result<Self> {
        match profile.provider.as_deref() {
            Some(p) if p.eq_ignore_ascii_case("openai") => Ok(Self::OpenAi),
            Some(p) if p.eq_ignore_ascii_case("deepseek") => Ok(Self::DeepSeek),
            Some(p) if p.eq_ignore_ascii_case("anthropic") => Ok(Self::Anthropic),
            Some(p) => Err(Error::Config(format!(
                "unknown provider '{p}' (expected openai | deepseek | anthropic)"
            ))),
            None => {
                if profile.base_url.contains("deepseek") {
                    Ok(Self::DeepSeek)
                } else if profile.base_url.contains("api.anthropic.com") {
                    // Only auto-detect Anthropic's first-party host. Third-party relays
                    // often have ambiguous URLs and should set provider="anthropic".
                    Ok(Self::Anthropic)
                } else {
                    Ok(Self::OpenAi)
                }
            }
        }
    }
}

/// Multi-protocol LLM client: OpenAI-compatible chat-completions (OpenAI,
/// DeepSeek, relays) plus Anthropic-native /v1/messages.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Default client; honors http(s)_proxy / no_proxy environment variables.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Client that bypasses any system proxy (tests against local mock servers).
    pub fn direct() -> Self {
        Self {
            http: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("reqwest client"),
        }
    }

    fn endpoint(profile: &Profile) -> Result<String> {
        Ok(match ProviderKind::from_profile(profile)? {
            ProviderKind::Anthropic => {
                format!("{}/v1/messages", profile.base_url.trim_end_matches('/'))
            }
            _ => format!(
                "{}/chat/completions",
                profile.base_url.trim_end_matches('/')
            ),
        })
    }

    fn anthropic_supports_disable_thinking(profile: &Profile) -> bool {
        !matches!(
            profile.model.as_str(),
            "claude-fable-5" | "claude-mythos-5"
        )
    }

    fn anthropic_default_max_tokens(json_mode: bool) -> u32 {
        // Anthropic-native requests require max_tokens. Keep translate bounded,
        // but give chat enough room for long replies / reasoning-heavy traces.
        if json_mode {
            16_000
        } else {
            64_000
        }
    }

    fn anthropic_messages(messages: &[ChatMessage]) -> (String, Vec<Value>) {
        let mut system_parts = vec![];
        let mut out = vec![];
        for m in messages {
            match m.role.as_str() {
                "system" => system_parts.push(m.content.clone()),
                "user" | "assistant" => out.push(json!({
                    "role": m.role,
                    "content": m.content,
                })),
                _ => {}
            }
        }
        (system_parts.join("\n\n"), out)
    }

    fn merge_selected_extra(
        body: &mut Value,
        extra: &Option<Map<String, Value>>,
        allowed_keys: &[&str],
    ) {
        let Some(extra) = extra else { return };
        for key in allowed_keys {
            if let Some(v) = extra.get(*key) {
                body[*key] = v.clone();
            }
        }
    }

    fn build_openai_body(
        profile: &Profile,
        messages: &[ChatMessage],
        stream: bool,
        json_mode: bool,
        kind: ProviderKind,
    ) -> Value {
        let mut body = json!({
            "model": profile.model,
            "messages": messages,
            "stream": stream,
        });
        if let Some(t) = profile.temperature {
            body["temperature"] = json!(t);
        }
        // Agent.rs resolves translate_effort/chat_effort into effort before calling us.
        // effort=Some(level) → thinking enabled; effort=None → thinking disabled.
        if matches!(kind, ProviderKind::DeepSeek) {
            body["thinking"] =
                json!({"type": if profile.effort.is_some() { "enabled" } else { "disabled" }});
        }
        if let Some(e) = &profile.effort {
            // OpenAI and most compatible APIs don't accept "xhigh" — normalise it.
            let level = if matches!(kind, ProviderKind::OpenAi) && e == "xhigh" {
                "high"
            } else {
                e.as_str()
            };
            body["reasoning_effort"] = json!(level);
        }
        if json_mode {
            body["response_format"] = json!({ "type": "json_object" });
        }
        if let Some(extra) = &profile.extra {
            for (k, v) in extra {
                body[k.as_str()] = v.clone();
            }
        }
        body
    }

    fn build_anthropic_body(
        profile: &Profile,
        messages: &[ChatMessage],
        stream: bool,
        json_mode: bool,
    ) -> Value {
        let (system, anthropic_messages) = Self::anthropic_messages(messages);
        let mut body = json!({
            "model": profile.model,
            "max_tokens": Self::anthropic_default_max_tokens(json_mode),
            "messages": anthropic_messages,
            "stream": stream,
        });
        if !system.is_empty() {
            body["system"] = json!(system);
        }

        let mut output_config = Map::new();
        if let Some(e) = &profile.effort {
            output_config.insert("effort".into(), json!(e));
            // Summarized thinking maps naturally onto Glossa's reasoning pane.
            body["thinking"] = json!({"type": "adaptive", "display": "summarized"});
        } else if Self::anthropic_supports_disable_thinking(profile) {
            // Fable/Mythos 5 reject explicit disabled thinking; other current
            // Anthropic models accept it and it preserves Glossa's "no thinking"
            // semantics when the effort fields are empty.
            body["thinking"] = json!({"type": "disabled"});
        }
        if json_mode {
            output_config.insert(
                "format".into(),
                json!({
                    "type": "json_schema",
                    "schema": schema::output_json_schema(),
                }),
            );
        }
        if !output_config.is_empty() {
            body["output_config"] = Value::Object(output_config);
        }

        // Anthropic-native does not accept the OpenAI-compatible body surface.
        // Only pass through an explicit, non-conflicting whitelist.
        Self::merge_selected_extra(
            &mut body,
            &profile.extra,
            &["max_tokens", "metadata", "stop_sequences", "service_tier"],
        );
        body
    }

    pub(crate) fn build_body(
        profile: &Profile,
        messages: &[ChatMessage],
        stream: bool,
        json_mode: bool,
    ) -> Result<Value> {
        Ok(match ProviderKind::from_profile(profile)? {
            ProviderKind::OpenAi => {
                Self::build_openai_body(profile, messages, stream, json_mode, ProviderKind::OpenAi)
            }
            ProviderKind::DeepSeek => {
                Self::build_openai_body(profile, messages, stream, json_mode, ProviderKind::DeepSeek)
            }
            ProviderKind::Anthropic => Self::build_anthropic_body(profile, messages, stream, json_mode),
        })
    }

    async fn post(&self, profile: &Profile, body: &Value) -> Result<reqwest::Response> {
        let kind = ProviderKind::from_profile(profile)?;
        let req = self.http.post(Self::endpoint(profile)?).json(body);
        let req = match kind {
            ProviderKind::Anthropic => req
                .header("x-api-key", profile.resolve_api_key()?)
                .header("anthropic-version", "2023-06-01"),
            _ => req.bearer_auth(profile.resolve_api_key()?),
        };
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp)
    }

    fn parse_openai_text(v: &Value) -> Result<String> {
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_owned)
            .ok_or(Error::EmptyResponse)
    }

    fn parse_anthropic_text(v: &Value) -> Result<String> {
        let text: String = v["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        if text.is_empty() {
            Err(Error::EmptyResponse)
        } else {
            Ok(text)
        }
    }

    /// Non-streaming completion; returns the assistant message content.
    pub async fn chat_once(
        &self,
        profile: &Profile,
        messages: &[ChatMessage],
        json_mode: bool,
    ) -> Result<String> {
        let body = Self::build_body(profile, messages, false, json_mode)?;
        let v: Value = self.post(profile, &body).await?.json().await?;
        match ProviderKind::from_profile(profile)? {
            ProviderKind::OpenAi | ProviderKind::DeepSeek => Self::parse_openai_text(&v),
            ProviderKind::Anthropic => Self::parse_anthropic_text(&v),
        }
    }

    fn parse_openai_stream_event(data: &str) -> Option<Result<Delta>> {
        if data.trim() == "[DONE]" {
            return None;
        }
        match serde_json::from_str::<Value>(data) {
            Ok(v) => {
                let delta = &v["choices"][0]["delta"];
                let reasoning = delta["reasoning_content"]
                    .as_str()
                    .or_else(|| delta["reasoning"].as_str())
                    .filter(|s| !s.is_empty());
                if let Some(r) = reasoning {
                    return Some(Ok(Delta::Reasoning(r.to_owned())));
                }
                delta["content"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| Ok(Delta::Content(s.to_owned())))
            }
            // Tolerate non-JSON keep-alives some providers send.
            Err(_) => None,
        }
    }

    fn parse_anthropic_stream_event(data: &str) -> Option<Result<Delta>> {
        // Some Anthropic relays keep the /v1/messages endpoint but stream
        // OpenAI-style `choices[].delta.*` payloads — accept those too.
        if let Some(ev) = Self::parse_openai_stream_event(data) {
            return Some(ev);
        }
        match serde_json::from_str::<Value>(data) {
            Ok(v) => match v["type"].as_str() {
                Some("content_block_start") => {
                    let block = &v["content_block"];
                    match block["type"].as_str() {
                        Some("thinking") => block["thinking"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| Ok(Delta::Reasoning(s.to_owned()))),
                        Some("text") => block["text"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| Ok(Delta::Content(s.to_owned()))),
                        _ => None,
                    }
                }
                Some("content_block_delta") => {
                    let delta = &v["delta"];
                    match delta["type"].as_str() {
                        Some("thinking_delta") => delta["thinking"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| Ok(Delta::Reasoning(s.to_owned()))),
                        Some("text_delta") => delta["text"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| Ok(Delta::Content(s.to_owned()))),
                        _ => {
                            // Relay compatibility: if the event envelope is Anthropic-like
                            // but the inner delta uses different field names, infer by
                            // the presence of reasoning/text fields instead of a strict type.
                            let reasoning = delta["thinking"]
                                .as_str()
                                .or_else(|| delta["reasoning_content"].as_str())
                                .or_else(|| delta["reasoning"].as_str())
                                .filter(|s| !s.is_empty());
                            if let Some(r) = reasoning {
                                return Some(Ok(Delta::Reasoning(r.to_owned())));
                            }
                            delta["text"]
                                .as_str()
                                .or_else(|| delta["content"].as_str())
                                .filter(|s| !s.is_empty())
                                .map(|s| Ok(Delta::Content(s.to_owned())))
                        }
                    }
                }
                _ => None,
            },
            Err(_) => None,
        }
    }

    /// Streaming completion; yields reasoning and content deltas separately
    /// (see [`Delta`]). Provider-specific wire formats are normalized here.
    pub async fn chat_stream(
        &self,
        profile: &Profile,
        messages: &[ChatMessage],
        json_mode: bool,
    ) -> Result<impl Stream<Item = Result<Delta>> + Send + Unpin> {
        let kind = ProviderKind::from_profile(profile)?;
        let body = Self::build_body(profile, messages, true, json_mode)?;
        let resp = self.post(profile, &body).await?;
        let stream = resp
            .bytes_stream()
            .eventsource()
            .filter_map(move |ev| {
                let kind = kind;
                async move {
                    match ev {
                        Ok(ev) => match kind {
                            ProviderKind::OpenAi | ProviderKind::DeepSeek => {
                                Self::parse_openai_stream_event(&ev.data)
                            }
                            ProviderKind::Anthropic => Self::parse_anthropic_stream_event(&ev.data),
                        },
                        Err(e) => Some(Err(Error::Stream(e.to_string()))),
                    }
                }
            });
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn deepseek_profile(effort: Option<&str>) -> Profile {
        Profile {
            name: "ds".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            api_key: "sk".into(),
            api_key_env: String::new(),
            model: "v4".into(),
            effort: effort.map(String::from),
            ..Default::default()
        }
    }

    fn openai_profile(effort: Option<&str>) -> Profile {
        Profile {
            name: "oa".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key: "sk".into(),
            api_key_env: String::new(),
            model: "gpt".into(),
            effort: effort.map(String::from),
            ..Default::default()
        }
    }

    fn anthropic_profile(effort: Option<&str>) -> Profile {
        Profile {
            name: "claude".into(),
            base_url: "https://api.anthropic.com".into(),
            api_key: "sk-ant".into(),
            api_key_env: String::new(),
            model: "claude-opus-4-8".into(),
            provider: Some("anthropic".into()),
            effort: effort.map(String::from),
            ..Default::default()
        }
    }

    fn anthropic_fable_profile() -> Profile {
        Profile {
            name: "fable".into(),
            base_url: "https://api.anthropic.com".into(),
            api_key: "sk-ant".into(),
            api_key_env: String::new(),
            model: "claude-fable-5".into(),
            provider: Some("anthropic".into()),
            effort: None,
            ..Default::default()
        }
    }

    fn messages() -> Vec<ChatMessage> {
        vec![ChatMessage::user("hi")]
    }

    #[test]
    fn no_effort_sends_thinking_disabled_without_reasoning() {
        let body = Client::build_body(&deepseek_profile(None), &messages(), false, false).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn effort_xhigh_sends_thinking_enabled_and_reasoning() {
        let body = Client::build_body(&deepseek_profile(Some("xhigh")), &messages(), false, false)
            .unwrap();
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["reasoning_effort"], "xhigh");
    }

    #[test]
    fn effort_low_sends_thinking_enabled_and_reasoning() {
        let body = Client::build_body(&deepseek_profile(Some("low")), &messages(), false, false)
            .unwrap();
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["reasoning_effort"], "low");
    }

    #[test]
    fn openai_no_thinking_field_and_xhigh_normalised() {
        let body = Client::build_body(&openai_profile(Some("xhigh")), &messages(), false, false)
            .unwrap();
        assert!(body.get("thinking").is_none());
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn openai_no_effort_sends_nothing() {
        let body = Client::build_body(&openai_profile(None), &messages(), false, false).unwrap();
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn explicit_provider_overrides_url_sniffing() {
        // deepseek-looking URL, but the user says it's an OpenAI-compatible provider
        let mut p = deepseek_profile(Some("xhigh"));
        p.provider = Some("openai".into());
        let body = Client::build_body(&p, &messages(), false, false).unwrap();
        assert!(body.get("thinking").is_none());
        assert_eq!(body["reasoning_effort"], "high");

        // aggregator URL without "deepseek", explicitly marked as deepseek
        let mut p = openai_profile(None);
        p.provider = Some("deepseek".into());
        let body = Client::build_body(&p, &messages(), false, false).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
    }

    #[test]
    fn anthropic_body_extracts_system_messages_and_uses_native_schema() {
        let msgs = vec![
            ChatMessage::system("sys"),
            ChatMessage::assistant("old"),
            ChatMessage::user("hi"),
        ];
        let body = Client::build_body(&anthropic_profile(Some("xhigh")), &msgs, true, true).unwrap();
        assert_eq!(body["system"], "sys");
        assert_eq!(body["messages"][0]["role"], "assistant");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["thinking"]["display"], "summarized");
        assert_eq!(body["output_config"]["effort"], "xhigh");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn anthropic_fable_omits_disabled_thinking_when_effort_absent() {
        let body = Client::build_body(&anthropic_fable_profile(), &messages(), false, false).unwrap();
        assert!(body.get("thinking").is_none());
    }

    #[tokio::test]
    async fn anthropic_chat_once_uses_messages_endpoint_and_headers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "sk-ant"))
            .and(header("anthropic-version", "2023-06-01"))
            .and(body_partial_json(json!({
                "system": "sys",
                "messages": [{"role": "user", "content": "hi"}],
                "stream": false,
                "thinking": {"type": "disabled"},
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "content": [{"type": "text", "text": "hello"}]
            })))
            .mount(&server)
            .await;

        let mut profile = anthropic_profile(None);
        profile.base_url = server.uri();
        let text = Client::direct()
            .chat_once(&profile, &[ChatMessage::system("sys"), ChatMessage::user("hi")], false)
            .await
            .unwrap();
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn anthropic_stream_parses_reasoning_and_text_deltas() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"让我想想\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"答案\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .mount(&server)
            .await;

        let mut profile = anthropic_profile(Some("high"));
        profile.base_url = server.uri();
        let mut stream = Client::direct()
            .chat_stream(&profile, &messages(), false)
            .await
            .unwrap();
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        assert!(matches!(first, Delta::Reasoning(r) if r == "让我想想"));
        assert!(matches!(second, Delta::Content(t) if t == "答案"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn anthropic_stream_accepts_content_block_start_shape() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"thinking\",\"thinking\":\"先想一下\"}}\n\n",
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"text\",\"text\":\"答案\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .mount(&server)
            .await;

        let mut profile = anthropic_profile(Some("high"));
        profile.base_url = server.uri();
        let mut stream = Client::direct()
            .chat_stream(&profile, &messages(), false)
            .await
            .unwrap();
        assert!(matches!(stream.next().await.unwrap().unwrap(), Delta::Reasoning(r) if r == "先想一下"));
        assert!(matches!(stream.next().await.unwrap().unwrap(), Delta::Content(t) if t == "答案"));
    }

    #[tokio::test]
    async fn anthropic_stream_accepts_relay_reasoning_field_shape() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"reasoning\":\"relay想法\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"content\":\"relay答案\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .mount(&server)
            .await;

        let mut profile = anthropic_profile(Some("high"));
        profile.base_url = server.uri();
        let mut stream = Client::direct()
            .chat_stream(&profile, &messages(), false)
            .await
            .unwrap();
        assert!(matches!(stream.next().await.unwrap().unwrap(), Delta::Reasoning(r) if r == "relay想法"));
        assert!(matches!(stream.next().await.unwrap().unwrap(), Delta::Content(t) if t == "relay答案"));
    }
}


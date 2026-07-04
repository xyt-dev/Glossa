use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};

use crate::config::Profile;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
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

/// OpenAI-compatible chat-completions client.
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

    fn endpoint(profile: &Profile) -> String {
        format!(
            "{}/chat/completions",
            profile.base_url.trim_end_matches('/')
        )
    }

    pub(crate) fn build_body(
        profile: &Profile,
        messages: &[ChatMessage],
        stream: bool,
        json_mode: bool,
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
        // Explicit `provider` beats URL sniffing (aggregator URLs can mislead).
        let is_deepseek = match profile.provider.as_deref() {
            Some(p) => p.eq_ignore_ascii_case("deepseek"),
            None => profile.base_url.contains("deepseek"),
        };
        if is_deepseek {
            body["thinking"] =
                json!({"type": if profile.effort.is_some() { "enabled" } else { "disabled" }});
        }
        if let Some(e) = &profile.effort {
            // OpenAI and most compatible APIs don't accept "xhigh" — normalise it.
            let level = if !is_deepseek && e == "xhigh" {
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

    async fn post(&self, profile: &Profile, body: &Value) -> Result<reqwest::Response> {
        let resp = self
            .http
            .post(Self::endpoint(profile))
            .bearer_auth(profile.resolve_api_key()?)
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp)
    }

    /// Non-streaming completion; returns the assistant message content.
    pub async fn chat_once(
        &self,
        profile: &Profile,
        messages: &[ChatMessage],
        json_mode: bool,
    ) -> Result<String> {
        let body = Self::build_body(profile, messages, false, json_mode);
        let v: Value = self.post(profile, &body).await?.json().await?;
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_owned)
            .ok_or(Error::EmptyResponse)
    }

    /// Streaming completion; yields content deltas (reasoning deltas are skipped).
    pub async fn chat_stream(
        &self,
        profile: &Profile,
        messages: &[ChatMessage],
        json_mode: bool,
    ) -> Result<impl Stream<Item = Result<String>> + Send + Unpin> {
        let body = Self::build_body(profile, messages, true, json_mode);
        let resp = self.post(profile, &body).await?;
        let stream = resp
            .bytes_stream()
            .eventsource()
            .filter_map(|ev| async move {
                match ev {
                    Ok(ev) => {
                        if ev.data.trim() == "[DONE]" {
                            return None;
                        }
                        match serde_json::from_str::<Value>(&ev.data) {
                            Ok(v) => v["choices"][0]["delta"]["content"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .map(|s| Ok(s.to_owned())),
                            // Tolerate non-JSON keep-alives some providers send.
                            Err(_) => None,
                        }
                    }
                    Err(e) => Some(Err(Error::Stream(e.to_string()))),
                }
            });
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

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

    fn messages() -> Vec<ChatMessage> {
        vec![ChatMessage::user("hi")]
    }

    #[test]
    fn no_effort_sends_thinking_disabled_without_reasoning() {
        let body = Client::build_body(&deepseek_profile(None), &messages(), false, false);
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn effort_xhigh_sends_thinking_enabled_and_reasoning() {
        let body = Client::build_body(&deepseek_profile(Some("xhigh")), &messages(), false, false);
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["reasoning_effort"], "xhigh");
    }

    #[test]
    fn effort_low_sends_thinking_enabled_and_reasoning() {
        let body = Client::build_body(&deepseek_profile(Some("low")), &messages(), false, false);
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["reasoning_effort"], "low");
    }

    #[test]
    fn openai_no_thinking_field_and_xhigh_normalised() {
        let body = Client::build_body(&openai_profile(Some("xhigh")), &messages(), false, false);
        assert!(body.get("thinking").is_none());
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn openai_no_effort_sends_nothing() {
        let body = Client::build_body(&openai_profile(None), &messages(), false, false);
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn explicit_provider_overrides_url_sniffing() {
        // deepseek-looking URL, but the user says it's an OpenAI-compatible provider
        let mut p = deepseek_profile(Some("xhigh"));
        p.provider = Some("openai".into());
        let body = Client::build_body(&p, &messages(), false, false);
        assert!(body.get("thinking").is_none());
        assert_eq!(body["reasoning_effort"], "high");

        // aggregator URL without "deepseek", explicitly marked as deepseek
        let mut p = openai_profile(None);
        p.provider = Some("deepseek".into());
        let body = Client::build_body(&p, &messages(), false, false);
        assert_eq!(body["thinking"]["type"], "disabled");
    }
}

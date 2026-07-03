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
        Self { role: "system".into(), content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
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
        Self { http: reqwest::Client::new() }
    }

    /// Client that bypasses any system proxy (tests against local mock servers).
    pub fn direct() -> Self {
        Self {
            http: reqwest::Client::builder().no_proxy().build().expect("reqwest client"),
        }
    }

    fn endpoint(profile: &Profile) -> String {
        format!("{}/chat/completions", profile.base_url.trim_end_matches('/'))
    }

    fn build_body(profile: &Profile, messages: &[ChatMessage], stream: bool, json_mode: bool) -> Value {
        let mut body = json!({
            "model": profile.model,
            "messages": messages,
            "stream": stream,
        });
        if let Some(t) = profile.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(e) = &profile.effort {
            body["reasoning_effort"] = json!(e);
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
        let stream = resp.bytes_stream().eventsource().filter_map(|ev| async move {
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

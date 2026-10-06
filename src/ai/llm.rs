// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Chat backends for the agent loop: OpenAI-compatible `/chat/completions` with
//! streaming and tool calls, plus a scripted backend for tests.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use futures_util::StreamExt;
use serde_json::{Value, json};

use crate::copilot::LlmConfig;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type TokenSink<'a> = &'a (dyn Fn(&str) + Send + Sync);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default)]
pub struct AssistantTurn {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

impl AssistantTurn {
    /// The assistant message to append to the transcript before tool results.
    pub fn to_message(&self) -> Value {
        if self.tool_calls.is_empty() {
            return json!({"role": "assistant", "content": self.content});
        }
        json!({
            "role": "assistant",
            "content": if self.content.is_empty() { Value::Null } else { Value::String(self.content.clone()) },
            "tool_calls": self.tool_calls.iter().map(|c| json!({
                "id": c.id,
                "type": "function",
                "function": {"name": c.name, "arguments": c.arguments}
            })).collect::<Vec<_>>()
        })
    }
}

pub trait ChatBackend: Send + Sync {
    fn model(&self) -> String;
    /// One assistant turn. Content tokens are passed to `on_token` as they arrive.
    fn complete<'a>(
        &'a self,
        messages: &'a [Value],
        tools: &'a [Value],
        on_token: TokenSink<'a>,
    ) -> BoxFuture<'a, anyhow::Result<AssistantTurn>>;
}

pub struct OpenAiBackend {
    pub cfg: LlmConfig,
}

impl OpenAiBackend {
    pub fn from_env() -> Option<Self> {
        crate::copilot::llm_config().map(|cfg| Self { cfg })
    }

    fn url(&self) -> String {
        if self.cfg.base_url.contains("/chat/completions") {
            self.cfg.base_url.clone()
        } else {
            format!(
                "{}/chat/completions",
                self.cfg.base_url.trim_end_matches('/')
            )
        }
    }

    fn request(&self, timeout_secs: u64) -> anyhow::Result<reqwest::RequestBuilder> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()?;
        let mut req = client.post(self.url());
        if !self.cfg.api_key.is_empty() {
            req = req.bearer_auth(&self.cfg.api_key);
        }
        for (k, v) in crate::copilot::extra_headers() {
            req = req.header(k, v);
        }
        Ok(req)
    }

    /// Non-streaming JSON-mode completion for structured tasks (intent, search, policy, RCA).
    pub async fn complete_json(&self, system: &str, user: &str) -> anyhow::Result<Value> {
        let body = json!({
            "model": self.cfg.model,
            "messages": [
                {"role": "system", "content": format!("{system}\nRespond with one JSON object only, no prose.")},
                {"role": "user", "content": user}
            ],
            "temperature": 0.1
        });
        let resp = self
            .request(crate::copilot::timeout_secs().max(60))?
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("LLM HTTP {}", resp.status());
        }
        let v: Value = resp.json().await?;
        let text = v
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        parse_json_object(text).ok_or_else(|| anyhow::anyhow!("model did not return JSON"))
    }
}

impl ChatBackend for OpenAiBackend {
    fn model(&self) -> String {
        self.cfg.model.clone()
    }

    fn complete<'a>(
        &'a self,
        messages: &'a [Value],
        tools: &'a [Value],
        on_token: TokenSink<'a>,
    ) -> BoxFuture<'a, anyhow::Result<AssistantTurn>> {
        Box::pin(async move {
            let mut body = json!({
                "model": self.cfg.model,
                "messages": messages,
                "temperature": 0.2,
                "stream": true
            });
            if !tools.is_empty() {
                body["tools"] = Value::Array(tools.to_vec());
                body["tool_choice"] = json!("auto");
            }
            let resp = self
                .request(crate::copilot::timeout_secs().max(120))?
                .json(&body)
                .send()
                .await?;
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!(
                    "LLM HTTP {status}: {}",
                    text.chars().take(300).collect::<String>()
                );
            }
            let is_sse = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|ct| ct.contains("event-stream"));
            if !is_sse {
                let v: Value = resp.json().await?;
                let turn =
                    turn_from_message(v.pointer("/choices/0/message").unwrap_or(&Value::Null));
                if !turn.content.is_empty() {
                    on_token(&turn.content);
                }
                return Ok(turn);
            }
            let mut acc = StreamAccumulator::default();
            let mut buf = String::new();
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                buf.push_str(&String::from_utf8_lossy(&chunk?));
                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf.drain(..=pos);
                    if acc.feed_line(&line, on_token) {
                        return Ok(acc.finish());
                    }
                }
            }
            if !buf.trim().is_empty() {
                acc.feed_line(buf.trim(), on_token);
            }
            Ok(acc.finish())
        })
    }
}

/// Folds OpenAI streaming deltas (content + indexed tool-call fragments) into one turn.
#[derive(Default)]
pub struct StreamAccumulator {
    content: String,
    calls: Vec<ToolCall>,
}

impl StreamAccumulator {
    /// Returns true on `[DONE]`.
    pub fn feed_line(&mut self, line: &str, on_token: TokenSink<'_>) -> bool {
        let Some(data) = line.strip_prefix("data:") else {
            return false;
        };
        let data = data.trim();
        if data == "[DONE]" {
            return true;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return false;
        };
        let Some(delta) = v.pointer("/choices/0/delta") else {
            return false;
        };
        if let Some(t) = delta.get("content").and_then(|c| c.as_str()) {
            if !t.is_empty() {
                self.content.push_str(t);
                on_token(t);
            }
        }
        if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tcs {
                let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                while self.calls.len() <= idx {
                    self.calls.push(ToolCall::default());
                }
                let slot = &mut self.calls[idx];
                if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                    slot.id = id.to_string();
                }
                if let Some(n) = tc.pointer("/function/name").and_then(|n| n.as_str()) {
                    slot.name.push_str(n);
                }
                if let Some(a) = tc.pointer("/function/arguments").and_then(|a| a.as_str()) {
                    slot.arguments.push_str(a);
                }
            }
        }
        false
    }

    pub fn finish(self) -> AssistantTurn {
        AssistantTurn {
            content: self.content,
            tool_calls: self
                .calls
                .into_iter()
                .filter(|c| !c.name.is_empty())
                .enumerate()
                .map(|(i, mut c)| {
                    if c.id.is_empty() {
                        c.id = format!("call_{i}");
                    }
                    c
                })
                .collect(),
        }
    }
}

fn turn_from_message(msg: &Value) -> AssistantTurn {
    let content = msg
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    let tool_calls = msg
        .get("tool_calls")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, tc)| {
                    Some(ToolCall {
                        id: tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("call_{i}")),
                        name: tc.pointer("/function/name")?.as_str()?.to_string(),
                        arguments: tc
                            .pointer("/function/arguments")
                            .and_then(|a| a.as_str())
                            .unwrap_or("{}")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    AssistantTurn {
        content,
        tool_calls,
    }
}

/// Pull the first JSON object out of model text (tolerates code fences and prose).
pub fn parse_json_object(text: &str) -> Option<Value> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        if v.is_object() {
            return Some(v);
        }
    }
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    serde_json::from_str(&t[start..=end]).ok()
}

/// Test backend that replays scripted turns and records what it was sent.
pub struct ScriptedBackend {
    turns: Mutex<Vec<AssistantTurn>>,
    pub seen: Mutex<Vec<Vec<Value>>>,
}

impl ScriptedBackend {
    pub fn new(mut turns: Vec<AssistantTurn>) -> Self {
        turns.reverse();
        Self {
            turns: Mutex::new(turns),
            seen: Mutex::new(Vec::new()),
        }
    }
}

impl ChatBackend for ScriptedBackend {
    fn model(&self) -> String {
        "scripted".into()
    }

    fn complete<'a>(
        &'a self,
        messages: &'a [Value],
        _tools: &'a [Value],
        on_token: TokenSink<'a>,
    ) -> BoxFuture<'a, anyhow::Result<AssistantTurn>> {
        Box::pin(async move {
            if let Ok(mut s) = self.seen.lock() {
                s.push(messages.to_vec());
            }
            let turn = self
                .turns
                .lock()
                .ok()
                .and_then(|mut t| t.pop())
                .unwrap_or_else(|| AssistantTurn {
                    content: "done".into(),
                    tool_calls: vec![],
                });
            if !turn.content.is_empty() {
                on_token(&turn.content);
            }
            Ok(turn)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_streamed_content_and_tool_calls() {
        let tokens = Mutex::new(String::new());
        let sink = |t: &str| tokens.lock().unwrap().push_str(t);
        let mut acc = StreamAccumulator::default();
        let lines = [
            r#"data: {"choices":[{"delta":{"content":"Check"}}]}"#,
            r#"data: {"choices":[{"delta":{"content":"ing"}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"get_vm","arguments":"{\"namesp"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ace\":\"default\"}"}}]}}]}"#,
            ": keep-alive",
            "data: [DONE]",
        ];
        let mut done = false;
        for l in lines {
            done = acc.feed_line(l, &sink);
        }
        assert!(done);
        let turn = acc.finish();
        assert_eq!(turn.content, "Checking");
        assert_eq!(*tokens.lock().unwrap(), "Checking");
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "get_vm");
        assert_eq!(turn.tool_calls[0].arguments, r#"{"namespace":"default"}"#);
    }

    #[test]
    fn non_streamed_message_parses_tool_calls() {
        let msg = json!({"content": null, "tool_calls": [
            {"id": "a", "function": {"name": "list_vms", "arguments": "{}"}}
        ]});
        let t = turn_from_message(&msg);
        assert_eq!(t.tool_calls[0].name, "list_vms");
        assert!(t.content.is_empty());
    }

    #[test]
    fn assistant_message_carries_tool_calls() {
        let t = AssistantTurn {
            content: String::new(),
            tool_calls: vec![ToolCall {
                id: "x".into(),
                name: "stop_vm".into(),
                arguments: "{}".into(),
            }],
        };
        let m = t.to_message();
        assert_eq!(m["tool_calls"][0]["function"]["name"], "stop_vm");
        assert!(m["content"].is_null());
    }

    #[test]
    fn parse_json_object_handles_fences_and_prose() {
        assert_eq!(
            parse_json_object("```json\n{\"a\":1}\n```").unwrap()["a"],
            1
        );
        assert_eq!(
            parse_json_object("Sure! {\"b\":2} hope that helps").unwrap()["b"],
            2
        );
        assert!(parse_json_object("no json").is_none());
    }

    #[tokio::test]
    async fn openai_backend_streams_from_compatible_server() {
        use axum::{Router, routing::post};
        let app = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                let body = concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
                    "data: {\"choices\":[{\"delta\":{\"content\":\" there\"}}]}\n\n",
                    "data: [DONE]\n\n"
                );
                ([("content-type", "text/event-stream")], body)
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let backend = OpenAiBackend {
            cfg: LlmConfig {
                base_url: format!("http://{addr}/v1"),
                api_key: String::new(),
                model: "fake".into(),
            },
        };
        let got = Mutex::new(Vec::<String>::new());
        let sink = |t: &str| got.lock().unwrap().push(t.to_string());
        let turn = backend
            .complete(&[json!({"role":"user","content":"hi"})], &[], &sink)
            .await
            .unwrap();
        assert_eq!(turn.content, "Hello there");
        assert_eq!(got.lock().unwrap().len(), 2);
    }
}

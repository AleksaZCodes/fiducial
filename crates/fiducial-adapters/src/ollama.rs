//! `ai = "ollama"` — the [`Ai`] contract on a local model server.
//!
//! Ollama serves an OpenAI-compatible endpoint (`/v1/chat/completions`), so this
//! is the same wire shape the OpenRouter adapter speaks, pointed at a host on the
//! same machine or LAN: no account, no key, no internet. The base URL is
//! `[ai] base_url` in `fiducial.toml` and, in the Compose file, `AI_BASE_URL`.
//!
//! The URL is plain `http://`. This crate builds `reqwest` without TLS, because
//! the model server sits beside the product server; point it across the
//! internet and it will refuse, rather than send prompts in the clear by
//! accident.

use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::ai::{
    Ai, AiError, ChatRequest, ChatResponse, Role, StopReason, StreamEvent, ToolCall, ToolChoice,
    Usage,
};
use crate::{BoxFuture, BoxStream};

pub struct OllamaAi {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaAi {
    /// `base_url` is the server's `/v1` root, e.g. `http://ollama:11434/v1`.
    pub fn new(base_url: &str, model: &str) -> Result<Self, AiError> {
        if model.is_empty() {
            return Err(AiError::ModelUnavailable {
                model: String::new(),
                reason: "no model: declare `[ai] model` (a name `ollama list` shows) and re-run \
                         `fid derive`"
                    .into(),
            });
        }
        if !base_url.starts_with("http://") {
            return Err(AiError::Rejected {
                reason: format!(
                    "ai = \"ollama\": base URL `{base_url}` is not plain http://. Ollama runs \
                     beside the product server; this adapter does not carry TLS."
                ),
            });
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        })
    }

    fn body(&self, request: &ChatRequest, stream: bool) -> Value {
        let mut messages: Vec<Value> = Vec::new();
        if let Some(system) = &request.system {
            messages.push(json!({"role": "system", "content": system}));
        }
        for m in &request.messages {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            let mut wire = json!({"role": role, "content": m.content});
            if let Some(id) = &m.tool_call_id {
                wire["tool_call_id"] = json!(id);
            }
            if !m.tool_calls.is_empty() {
                wire["tool_calls"] = m
                    .tool_calls
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "type": "function",
                            "function": {"name": c.name, "arguments": c.arguments},
                        })
                    })
                    .collect();
            }
            messages.push(wire);
        }
        let mut body = json!({
            "model": request.model.clone().unwrap_or_else(|| self.model.clone()),
            "messages": messages,
        });
        if stream {
            body["stream"] = json!(true);
            body["stream_options"] = json!({"include_usage": true});
        }
        if !request.tools.is_empty() {
            body["tools"] = request
                .tools
                .iter()
                .map(|t| {
                    // `parameters` is a JSON Schema carried as text; an
                    // unparsable one is sent as an empty object schema rather
                    // than failing the call over a description.
                    let params: Value = serde_json::from_str(&t.parameters)
                        .unwrap_or_else(|_| json!({"type": "object", "properties": {}}));
                    json!({"type": "function", "function": {
                        "name": t.name, "description": t.description, "parameters": params}})
                })
                .collect();
            body["tool_choice"] = json!(match request.tool_choice.unwrap_or(ToolChoice::Auto) {
                ToolChoice::Auto => "auto",
                ToolChoice::None => "none",
                ToolChoice::Required => "required",
            });
        }
        if let Some(n) = request.max_tokens {
            body["max_tokens"] = json!(n);
        }
        if let Some(t) = request.temperature {
            body["temperature"] = json!(t);
        }
        body
    }

    async fn post(
        &self,
        request: &ChatRequest,
        stream: bool,
    ) -> Result<reqwest::Response, AiError> {
        let res = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&self.body(request, stream))
            .send()
            .await
            .map_err(|e| {
                AiError::Other(format!(
                    "OllamaAi: cannot reach {}: {e}. Is the `ollama` service up?",
                    self.base_url
                ))
            })?;
        if res.status().is_success() {
            return Ok(res);
        }
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        let message = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or(text);
        Err(classify(
            status,
            &message,
            request.model.as_deref().unwrap_or(&self.model),
        ))
    }
}

fn classify(status: u16, message: &str, model: &str) -> AiError {
    let lower = message.to_ascii_lowercase();
    if lower.contains("context") && (lower.contains("length") || lower.contains("exceed")) {
        AiError::ContextLengthExceeded {
            model: model.into(),
        }
    } else if status == 404 || lower.contains("not found") {
        // Ollama answers an unpulled model with 404 and `model "x" not found,
        // try pulling it first`.
        AiError::ModelUnavailable {
            model: model.into(),
            reason: message.into(),
        }
    } else if status == 429 {
        AiError::RateLimited {
            retry_after_secs: 1,
        }
    } else {
        AiError::Rejected {
            reason: format!("HTTP {status}: {message}"),
        }
    }
}

fn stop_reason(finish: Option<&str>) -> StopReason {
    match finish {
        Some("stop") => StopReason::Stop,
        Some("length") => StopReason::Length,
        Some("tool_calls") => StopReason::ToolCalls,
        _ => StopReason::Other,
    }
}

fn usage_of(v: &Value) -> Usage {
    Usage {
        input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
    }
}

fn tool_calls_of(v: &Value) -> Vec<ToolCall> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|c| ToolCall {
                    id: c["id"].as_str().unwrap_or_default().to_string(),
                    name: c["function"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    // Some servers return arguments as an object, not text.
                    arguments: match &c["function"]["arguments"] {
                        Value::String(s) => s.clone(),
                        Value::Null => String::new(),
                        other => other.to_string(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

impl Ai for OllamaAi {
    fn chat<'a>(
        &'a self,
        request: &'a ChatRequest,
    ) -> BoxFuture<'a, Result<ChatResponse, AiError>> {
        Box::pin(async move {
            let res = self.post(request, false).await?;
            let data: Value = res
                .json()
                .await
                .map_err(|e| AiError::Other(format!("OllamaAi: unreadable response: {e}")))?;
            let choice = data["choices"]
                .get(0)
                .ok_or_else(|| AiError::Other("OllamaAi: response had no choices".into()))?;
            Ok(ChatResponse {
                text: choice["message"]["content"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                tool_calls: tool_calls_of(&choice["message"]["tool_calls"]),
                stop_reason: stop_reason(choice["finish_reason"].as_str()),
                usage: usage_of(&data),
                model: data["model"].as_str().unwrap_or(&self.model).to_string(),
            })
        })
    }

    fn stream<'a>(
        &'a self,
        request: &'a ChatRequest,
    ) -> BoxStream<'a, Result<StreamEvent, AiError>> {
        Box::pin(
            futures_util::stream::once(async move { self.post(request, true).await }).flat_map(
                |res| match res {
                    Ok(res) => events(res),
                    Err(e) => Box::pin(futures_util::stream::once(std::future::ready(Err(e)))),
                },
            ),
        )
    }
}

type Events =
    std::pin::Pin<Box<dyn futures_core::Stream<Item = Result<StreamEvent, AiError>> + Send>>;

/// Server-sent events in, [`StreamEvent`]s out.
///
/// Text arrives as it is generated. A tool call arrives as fragments (an id and
/// a name once, the arguments in pieces), so they are assembled by index and
/// emitted whole, just before `Done` — the contract's promise.
fn events(res: reqwest::Response) -> Events {
    struct State {
        bytes: std::pin::Pin<Box<dyn futures_core::Stream<Item = reqwest::Result<Vec<u8>>> + Send>>,
        buf: String,
        calls: Vec<ToolCall>,
        finish: Option<String>,
        usage: Usage,
        queue: std::collections::VecDeque<Result<StreamEvent, AiError>>,
        done: bool,
    }

    impl State {
        fn finish(&mut self) {
            if self.done {
                return;
            }
            self.done = true;
            for c in std::mem::take(&mut self.calls) {
                self.queue.push_back(Ok(StreamEvent::ToolCall(c)));
            }
            self.queue.push_back(Ok(StreamEvent::Done {
                stop_reason: stop_reason(self.finish.as_deref()),
                usage: self.usage,
            }));
        }

        fn line(&mut self, line: &str) {
            let Some(data) = line.strip_prefix("data:") else {
                return;
            };
            let data = data.trim();
            if data == "[DONE]" {
                self.finish();
                return;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else {
                return;
            };
            if v["usage"].is_object() {
                self.usage = usage_of(&v);
            }
            let Some(choice) = v["choices"].get(0) else {
                return;
            };
            if let Some(t) = choice["delta"]["content"].as_str() {
                if !t.is_empty() {
                    self.queue.push_back(Ok(StreamEvent::Text(t.to_string())));
                }
            }
            if let Some(frags) = choice["delta"]["tool_calls"].as_array() {
                for f in frags {
                    let idx = f["index"].as_u64().unwrap_or(0) as usize;
                    while self.calls.len() <= idx {
                        self.calls.push(ToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: String::new(),
                        });
                    }
                    let c = &mut self.calls[idx];
                    if let Some(id) = f["id"].as_str() {
                        c.id = id.to_string();
                    }
                    if let Some(n) = f["function"]["name"].as_str() {
                        c.name.push_str(n);
                    }
                    match &f["function"]["arguments"] {
                        Value::String(a) => c.arguments.push_str(a),
                        Value::Null => {}
                        other => c.arguments.push_str(&other.to_string()),
                    }
                }
            }
            if let Some(r) = choice["finish_reason"].as_str() {
                self.finish = Some(r.to_string());
            }
        }
    }

    let state = State {
        bytes: Box::pin(res.bytes_stream().map(|r| r.map(|b| b.to_vec()))),
        buf: String::new(),
        calls: Vec::new(),
        finish: None,
        usage: Usage::default(),
        queue: std::collections::VecDeque::new(),
        done: false,
    };
    Box::pin(futures_util::stream::unfold(state, |mut st| async move {
        loop {
            if let Some(ev) = st.queue.pop_front() {
                return Some((ev, st));
            }
            if st.done {
                return None;
            }
            match st.bytes.next().await {
                Some(Ok(chunk)) => {
                    st.buf.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(n) = st.buf.find('\n') {
                        let line: String = st.buf.drain(..=n).collect();
                        st.line(line.trim_end());
                    }
                }
                Some(Err(e)) => {
                    st.done = true;
                    st.queue
                        .push_back(Err(AiError::Other(format!("OllamaAi: stream broke: {e}"))));
                }
                // The server closed without `[DONE]`: still end the stream properly.
                None => st.finish(),
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Message;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serve one canned HTTP response, and hand back the request that arrived.
    async fn serve(
        content_type: &'static str,
        body: String,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let h = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(h) = text.find("\r\n\r\n") {
                    let len = text[..h]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if got.len() >= h + 4 + len {
                        break;
                    }
                }
            }
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&got).to_string()
        });
        (url, h)
    }

    fn request() -> ChatRequest {
        ChatRequest {
            messages: vec![Message::user("hi")],
            system: Some("be brief".into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn chat_sends_the_openai_shape_and_reads_the_answer() {
        let body = r#"{"model":"llama3","choices":[{"message":{"content":"hello"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
        let (url, sent) = serve("application/json", body.into()).await;
        let ai = OllamaAi::new(&url, "llama3").unwrap();
        let r = ai.chat(&request()).await.unwrap();
        assert_eq!(r.text, "hello");
        assert_eq!(r.stop_reason, StopReason::Stop);
        assert_eq!(
            r.usage,
            Usage {
                input_tokens: 3,
                output_tokens: 1
            }
        );
        let req = sent.await.unwrap();
        assert!(req.starts_with("POST /v1/chat/completions"), "{req}");
        assert!(
            req.contains("\"role\":\"system\"") && req.contains("be brief"),
            "{req}"
        );
        assert!(req.contains("\"model\":\"llama3\""), "{req}");
    }

    #[tokio::test]
    async fn a_stream_yields_text_then_whole_tool_calls_then_done() {
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c1\",\"function\":{\"name\":\"get\",\"arguments\":\"{\\\"a\\\"\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\":1}\"}}]},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\n",
            "data: [DONE]\n\n"
        );
        let (url, _) = serve("text/event-stream", sse.into()).await;
        let ai = OllamaAi::new(&url, "llama3").unwrap();
        let req = request();
        let evs: Vec<_> = ai.stream(&req).collect().await;
        let evs: Vec<StreamEvent> = evs.into_iter().map(Result::unwrap).collect();
        assert_eq!(evs[0], StreamEvent::Text("Hel".into()));
        assert_eq!(evs[1], StreamEvent::Text("lo".into()));
        assert_eq!(
            evs[2],
            StreamEvent::ToolCall(ToolCall {
                id: "c1".into(),
                name: "get".into(),
                arguments: "{\"a\":1}".into()
            })
        );
        assert_eq!(
            evs[3],
            StreamEvent::Done {
                stop_reason: StopReason::ToolCalls,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 2
                }
            }
        );
        assert_eq!(evs.len(), 4);
    }

    #[test]
    fn a_remote_or_tls_url_and_a_missing_model_are_refused() {
        let e = OllamaAi::new("https://ollama.example.com/v1", "m")
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains("not plain http://"), "{e}");
        let e = OllamaAi::new("http://ollama:11434/v1", "")
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains("declare `[ai] model`"), "{e}");
    }

    #[test]
    fn an_unpulled_model_is_named() {
        let e = classify(404, "model \"x\" not found, try pulling it first", "x");
        assert!(matches!(e, AiError::ModelUnavailable { .. }), "{e:?}");
    }
}

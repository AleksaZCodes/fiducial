//! `ai = "ollama"` against a real model server.
//!
//! The unit tests in `src/ollama.rs` serve canned HTTP, which proves the adapter
//! speaks the wire format it was written against and nothing about what Ollama
//! sends. This runs the adapter against a real Ollama and a real (small) model.
//!
//! It is `#[ignore]` because it needs a server and a downloaded model, and it
//! says so rather than passing quietly without one. Run it with:
//!
//! ```sh
//! ollama pull qwen2.5:0.5b
//! OLLAMA_BASE_URL=http://127.0.0.1:11434/v1 OLLAMA_MODEL=qwen2.5:0.5b \
//!   cargo test -p fiducial-adapters --features ollama --test ollama_real -- --ignored
//! ```
//!
//! The model is small enough to run on a CPU, so these assert on the shape of the
//! answer (it arrived, it streamed, it stopped, it counted tokens), never on its
//! wording.
#![cfg(feature = "ollama")]

use fiducial_adapters::ai::{Ai, AiError, ChatRequest, Message, StopReason, StreamEvent};
use fiducial_adapters::ollama::OllamaAi;
use futures_util::StreamExt;

fn target() -> (String, String) {
    let url = std::env::var("OLLAMA_BASE_URL")
        .expect("set OLLAMA_BASE_URL (e.g. http://127.0.0.1:11434/v1)");
    let model = std::env::var("OLLAMA_MODEL").expect("set OLLAMA_MODEL (e.g. qwen2.5:0.5b)");
    (url, model)
}

fn ask(text: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![Message::user(text)],
        max_tokens: Some(48),
        temperature: Some(0.0),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "needs a running Ollama and a pulled model (see the module docs)"]
async fn a_real_model_answers_and_reports_what_it_used() {
    let (url, model) = target();
    let ai = OllamaAi::new(&url, &model).unwrap();
    let r = ai
        .chat(&ask("Reply with the single word: ready"))
        .await
        .unwrap();
    assert!(!r.text.trim().is_empty(), "an empty completion");
    assert!(
        r.usage.input_tokens > 0 && r.usage.output_tokens > 0,
        "{:?}",
        r.usage
    );
    assert!(
        matches!(r.stop_reason, StopReason::Stop | StopReason::Length),
        "{:?}",
        r.stop_reason
    );
    assert!(
        r.model.contains(model.split(':').next().unwrap()),
        "served by {}",
        r.model
    );
}

#[tokio::test]
#[ignore = "needs a running Ollama and a pulled model (see the module docs)"]
async fn a_real_stream_concatenates_to_text_and_ends_with_done() {
    let (url, model) = target();
    let ai = OllamaAi::new(&url, &model).unwrap();
    let req = ask("Count from one to five in words.");
    let mut s = ai.stream(&req);
    let (mut text, mut chunks, mut done) = (String::new(), 0, None);
    while let Some(ev) = s.next().await {
        match ev.unwrap() {
            StreamEvent::Text(t) => {
                chunks += 1;
                text.push_str(&t);
            }
            StreamEvent::Done { stop_reason, usage } => done = Some((stop_reason, usage)),
            StreamEvent::ToolCall(_) => panic!("no tools were declared"),
        }
    }
    assert!(
        chunks > 1,
        "a stream that arrived as one chunk is not streaming ({chunks})"
    );
    assert!(!text.trim().is_empty());
    let (_, usage) = done.expect("a successful stream always ends with Done");
    assert!(usage.output_tokens > 0, "{usage:?}");
}

#[tokio::test]
#[ignore = "needs a running Ollama and a pulled model (see the module docs)"]
async fn a_model_that_is_not_there_is_named_not_swallowed() {
    let (url, _) = target();
    let ai = OllamaAi::new(&url, "no-such-model:0b").unwrap();
    match ai.chat(&ask("hi")).await {
        Err(AiError::ModelUnavailable { model, .. }) => assert!(model.contains("no-such-model")),
        Err(other) => panic!("expected ModelUnavailable, got {other}"),
        Ok(r) => panic!("a missing model answered: {r:?}"),
    }
}

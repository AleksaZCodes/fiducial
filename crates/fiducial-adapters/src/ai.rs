//! AI contract — conversational and agentic language-model calls.
//!
//! # Why this contract targets a gateway, not a model vendor
//!
//! The obvious shape for this contract was one adapter per model vendor —
//! `anthropic`, `openai`, `workers-ai` — behind their narrowest shared
//! interface. That shape does not work, and the reason is not a detail:
//!
//! - Anthropic puts `system` at the top level; OpenAI makes it a message role.
//! - Anthropic has content blocks; OpenAI has a string or a parts array.
//! - Streaming event shapes differ substantially between the two.
//! - The surface moves fast — thinking config, `tool_choice` and prefill all
//!   changed shape inside a year.
//!
//! Intersect those by hand and the contract is too thin to write an agent
//! against. Superset them and you have picked a vendor without admitting it,
//! which is `MISSION.md` principle 6 broken in the file that implements it.
//!
//! So the contract targets **one** shape — an AI gateway's — and *model
//! choice becomes a string in a declaration* rather than an adapter per
//! vendor. A gateway already does the cross-vendor normalization and, more
//! importantly, maintains it; vendor drift becomes the thing you are paying
//! the gateway for. `openrouter` is the first implementation.
//!
//! # Embeddings are deliberately absent
//!
//! Turning product data into vectors is a real second use, with its own
//! method shape, and it pairs with a vector store this platform does not
//! have. Adding `embed()` here before anything can store what it returns
//! would put a method on every implementation that no product can complete a
//! task with. It lands when it has a consumer.
//!
//! # Rust has the contract; `openrouter` is TypeScript-only
//!
//! Same boundary as `turnstile` and `supabase` (auth): a plain HTTPS API that
//! Rust could reach, with no Rust-side consumer that would use it. Every AI
//! call in the product shapes this platform scaffolds happens in a Worker or
//! a server route. The trait is defined here so the contract is one contract
//! in both languages — see `packages/adapters/src/ai.ts` for the half that
//! has a vendor behind it.

use thiserror::Error;

use crate::{BoxFuture, BoxStream};

/// Who authored a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Instructions for the model. Prefer [`ChatRequest::system`] — see its
    /// doc for why this variant still exists.
    System,
    User,
    Assistant,
    /// The result of a tool the assistant asked to call. Carries the
    /// `tool_call_id` it answers.
    Tool,
}

/// A tool invocation the model asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Vendor-assigned id. A [`Role::Tool`] message answers it by id, not by
    /// name — a model may call the same tool twice in one turn.
    pub id: String,
    pub name: String,
    /// Arguments as a JSON **string**, not a parsed value.
    ///
    /// This crate is dependency-light on purpose and has no JSON type. More
    /// to the point, the caller has to validate these against the schema it
    /// declared regardless of who parses them: a model can emit arguments
    /// that are valid JSON and still wrong.
    pub arguments: String,
}

/// One turn in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    /// Text content. Empty for an assistant turn that was only tool calls.
    pub content: String,
    /// Set on a [`Role::Tool`] message: which call this answers.
    pub tool_call_id: Option<String>,
    /// Set on a [`Role::Assistant`] message replayed back into a follow-up
    /// request: the calls that turn asked for.
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    /// A plain user turn — the common case, without five `None`s at the call
    /// site.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    /// A plain assistant turn.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    /// The result of a tool call, answering it by id.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
        }
    }
}

/// A tool the model may call.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    /// What the tool does. The model reads this to decide whether to call it,
    /// so it is part of the prompt, not documentation.
    pub description: String,
    /// JSON Schema for the arguments, as a JSON string — same reason
    /// [`ToolCall::arguments`] is a string.
    pub parameters: String,
}

/// Whether the model must call a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolChoice {
    /// The model decides. The default when tools are present.
    Auto,
    /// Tools are declared but must not be called this turn.
    None,
    /// The model must call one of the declared tools.
    Required,
}

/// A request for a completion.
#[derive(Debug, Clone, Default)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    /// System prompt, top-level.
    ///
    /// Carried top-level rather than as a first [`Role::System`] message
    /// because the two directions are not equally easy: a top-level system
    /// prompt lowers to a system message in one line, and recovering "which
    /// of these messages was the system prompt" from a list does not always
    /// work. Vendors that want a message role get one from the adapter.
    pub system: Option<String>,
    pub tools: Vec<ToolDefinition>,
    /// Defaults to [`ToolChoice::Auto`] when `tools` is non-empty.
    pub tool_choice: Option<ToolChoice>,
    /// Override the model for this one call.
    ///
    /// Normally `None`: the model is a declaration (`[ai] model` in
    /// `fiducial.toml`), derived into the generated factory and gated like
    /// every other derived fact. This exists because an agent that uses a
    /// small model for classification and a large one for the final answer is
    /// a normal agent, not an exotic one — and forcing that product to
    /// construct a second adapter would make the declaration a lie rather
    /// than a default.
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The model finished its turn.
    Stop,
    /// `max_tokens` was reached — the response is truncated.
    Length,
    /// The model is waiting on tool results.
    ToolCalls,
    /// Anything else the gateway reported, including content filtering.
    Other,
}

/// Tokens billed for one call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// A completed response.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    /// Non-empty exactly when `stop_reason` is [`StopReason::ToolCalls`].
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: StopReason,
    pub usage: Usage,
    /// The model that actually served the call.
    ///
    /// Reported rather than assumed: a gateway may route to a different model
    /// than the one requested (a fallback, a provider outage, an alias
    /// resolving to a dated snapshot), and a product logging what it *asked
    /// for* is logging a guess.
    pub model: String,
}

/// One event in a streamed response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// A chunk of assistant text. Concatenating every `Text` in order yields
    /// [`ChatResponse::text`].
    Text(String),
    /// A complete tool call.
    ///
    /// Emitted whole, not as argument fragments: gateways stream tool
    /// arguments as partial JSON, and a partial JSON string is not something
    /// a consumer can do anything with except buffer it. The adapter does
    /// that buffering once so every consumer does not.
    ToolCall(ToolCall),
    /// The final event. Always emitted on a successful stream.
    Done {
        stop_reason: StopReason,
        usage: Usage,
    },
}

/// Errors an [`Ai`] implementation may return.
#[derive(Debug, Error)]
pub enum AiError {
    /// The gateway refused the request — bad key, filtered content, malformed
    /// tool schema.
    #[error("request rejected by provider: {reason}")]
    Rejected { reason: String },
    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    /// The conversation is longer than the model's context window.
    ///
    /// Its own variant because the caller's remedy is specific and
    /// programmable — drop or summarize old turns and retry — where the
    /// remedy for [`AiError::Rejected`] is to read the message.
    #[error("context length exceeded for model `{model}`")]
    ContextLengthExceeded { model: String },
    /// The requested model does not exist, or the account cannot reach it.
    #[error("model `{model}` is not available: {reason}")]
    ModelUnavailable { model: String, reason: String },
    #[error("{0}")]
    Other(String),
}

/// Conversational and agentic language-model calls, through a gateway.
pub trait Ai: Send + Sync {
    /// Run a completion to the end and return it.
    fn chat<'a>(&'a self, request: &'a ChatRequest)
        -> BoxFuture<'a, Result<ChatResponse, AiError>>;

    /// Run a completion, yielding events as they arrive.
    ///
    /// A separate method rather than a flag on [`ChatRequest`] because the
    /// return types genuinely differ, and a `stream: bool` that changes what
    /// comes back is a return type in disguise.
    fn stream<'a>(
        &'a self,
        request: &'a ChatRequest,
    ) -> BoxStream<'a, Result<StreamEvent, AiError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op AI — every call fails with [`AiError::Rejected`] naming the fix.
///
/// **This is the second `None*` that fails rather than succeeding**, after
/// [`crate::NoneAuth`], and for a related reason. A no-op send ([`crate::NoneEmail`])
/// or a no-op enqueue is indistinguishable from the real thing at the call
/// site: the caller wanted an effect elsewhere and does not read a result. A
/// completion *is* the result. Returning an empty one turns "no AI vendor is
/// selected" into a blank answer in a product's UI, which reads as a model
/// bug and is debugged as one.
///
/// It still costs nothing to wire in: constructing it is free, it sits in
/// `AdapterSet` from the first commit, and nothing fails until something
/// actually asks for a completion — at which point failing is the honest
/// answer.
pub struct NoneAi;

const NONE_MESSAGE: &str = "ai = \"none\": no AI vendor is selected. Set \
                            `[adapters] ai = \"openrouter\"` and `[ai] model` \
                            in fiducial.toml, then re-run `fid derive`.";

impl Ai for NoneAi {
    fn chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
    ) -> BoxFuture<'a, Result<ChatResponse, AiError>> {
        Box::pin(std::future::ready(Err(AiError::Rejected {
            reason: NONE_MESSAGE.to_string(),
        })))
    }

    fn stream<'a>(
        &'a self,
        _request: &'a ChatRequest,
    ) -> BoxStream<'a, Result<StreamEvent, AiError>> {
        Box::pin(Once(Some(Err(AiError::Rejected {
            reason: NONE_MESSAGE.to_string(),
        }))))
    }
}

/// A stream that yields one item and ends.
///
/// Hand-written rather than `futures_util::stream::once` so this crate depends
/// on `futures-core` — the `Stream` trait alone — and not on a combinator
/// library every consumer would then link. Six lines is cheaper than that.
struct Once<T>(Option<T>);

impl<T: Unpin> futures_core::Stream for Once<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        std::task::Poll::Ready(self.0.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ai() -> Box<dyn Ai> {
        Box::new(NoneAi)
    }

    fn request() -> ChatRequest {
        ChatRequest {
            messages: vec![Message::user("hello")],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn chat_fails_loudly_rather_than_returning_an_empty_completion() {
        let err = ai().chat(&request()).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no AI vendor is selected"), "{msg}");
        assert!(
            msg.contains("fid derive"),
            "the message names the fix: {msg}"
        );
    }

    /// The stream half fails the same way, and ends.
    ///
    /// A stream that errors and then hangs is worse than one that returns an
    /// error, so the termination is asserted, not just the error.
    #[tokio::test]
    async fn stream_fails_loudly_and_terminates() {
        use futures_util::StreamExt;

        let ai = ai();
        let req = request();
        let mut events = ai.stream(&req);
        let first = events.next().await.expect("one event");
        assert!(matches!(first, Err(AiError::Rejected { .. })));
        assert!(events.next().await.is_none(), "the stream must end");
    }

    #[test]
    fn none_ai_is_object_safe() {
        let _: Box<dyn Ai> = Box::new(NoneAi);
    }

    #[test]
    fn a_tool_result_answers_a_call_by_id_not_by_name() {
        let m = Message::tool_result("call_1", "{\"ok\":true}");
        assert_eq!(m.role, Role::Tool);
        assert_eq!(m.tool_call_id.as_deref(), Some("call_1"));
    }
}

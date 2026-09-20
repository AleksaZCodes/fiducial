//! System One contract — fast, typed, probabilistic decisions.
//!
//! System One models (TypeSafe AI's Jev) return typed answers with calibrated
//! confidence rather than text. One request sends named questions about a piece
//! of state, and every question is answered in a single round-trip.
//!
//! # Three question types
//!
//! - **Noul** — yes/no; returns a probability on [0, 1]. Values near 0.5
//!   mean genuine uncertainty, not medium intensity.
//! - **Choice** — pick one of N options; returns the chosen key plus a
//!   probability distribution over all options.
//! - **Score** — position on an ordered scale; returns a continuous score
//!   plus a distribution over discrete levels.
//!
//! # Why a separate contract from `Ai`
//!
//! `Ai` takes a turn history and returns text or tool calls. `SystemOne`
//! takes a flat state value and named question schemas and returns typed
//! answers with probabilities — not text. The two have no shared surface;
//! a shared trait would pick the wrong shape for one of them.
//!
//! # Rust has the contract; the vendors are TypeScript-only
//!
//! Same boundary as `ai` / `openrouter`: both decisions endpoints are plain
//! HTTPS and Rust could reach them, but every product call happens in a Worker
//! or server route, not in a Rust binary. The trait lives here so the contract
//! is one contract in both languages. See `packages/adapters/src/system-one.ts`
//! for the half with vendors (`OpenRouterSystemOne`, `TypeSafeSystemOne`)
//! behind it.

use std::collections::HashMap;

use thiserror::Error;

use crate::BoxFuture;

// `state` is a JSON string, not a parsed value — same policy as
// `ToolCall.arguments` in `ai.rs`: this crate has no JSON type, and the
// caller has to serialize anyway. A vendor that needs to parse it does so once,
// at the boundary.

// ── Question types ────────────────────────────────────────────────────────────

/// A yes/no question.
#[derive(Debug, Clone)]
pub struct NoulQuestion {
    pub instructions: String,
    /// Optional gloss on what a yes means. Optional per the API.
    pub true_criteria: Option<String>,
    /// Optional gloss on what a no means.
    pub false_criteria: Option<String>,
}

/// Pick one of N options.
#[derive(Debug, Clone)]
pub struct ChoiceQuestion {
    pub instructions: String,
    /// Option name → description. `None` when the key needs no gloss.
    /// Maximum 255 options.
    pub criteria: HashMap<String, Option<String>>,
}

/// Position on an ordered scale.
#[derive(Debug, Clone)]
pub struct ScoreQuestion {
    pub instructions: String,
    /// Levels from lowest to highest. At least 2, at most 10.
    pub criteria: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Question {
    Noul(NoulQuestion),
    Choice(ChoiceQuestion),
    Score(ScoreQuestion),
}

// ── Answer types ──────────────────────────────────────────────────────────────

/// Probability of "true" on [0, 1].
///
/// Carries no `confidence`: the value is already the distribution over two
/// outcomes, so a separate concentration measure would restate it.
#[derive(Debug, Clone)]
pub struct NoulAnswer {
    pub noul: f64,
}

/// The chosen option key plus a probability distribution over all options.
#[derive(Debug, Clone)]
pub struct ChoiceAnswer {
    pub choice: String,
    pub probabilities: HashMap<String, f64>,
    /// Concentration of the distribution — 1 is certain, 0 is uniform.
    pub confidence: f64,
}

/// Continuous position between discrete levels.
#[derive(Debug, Clone)]
pub struct ScoreAnswer {
    pub score: f64,
    pub legend: HashMap<String, String>,
    pub probabilities: HashMap<String, f64>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum Answer {
    Noul(NoulAnswer),
    Choice(ChoiceAnswer),
    Score(ScoreAnswer),
}

// ── Request / response ────────────────────────────────────────────────────────

/// A batch of questions about one piece of state.
///
/// Questions run in parallel and cannot see each other's answers. Batch
/// independent questions; do not batch questions whose answer depends on
/// another question's answer.
#[derive(Debug, Clone)]
pub struct SystemOneRequest {
    /// JSON-encoded state the model reasons about for every question.
    ///
    /// A JSON string (not a nested string) — serialize with `serde_json` or
    /// equivalent before constructing the request. The vendor sends this
    /// verbatim; no extra quoting.
    pub state: String,
    /// Named questions — each key names the answer in the response.
    pub questions: HashMap<String, Question>,
    /// Override the model for this one call.
    pub model: Option<String>,
    /// Group related requests for observability (gateway only, <=256 chars).
    /// Never sent to the model, so it cannot affect an answer.
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SystemOneResponse {
    pub answers: HashMap<String, Answer>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Billed cost in USD. Reported by the gateway; absent on the direct API.
    pub cost: Option<f64>,
    /// The model that actually served the call.
    pub model: String,
    /// Gateway request id, when the vendor returns one.
    pub id: Option<String>,
    /// Upstream provider name, when the vendor reports one.
    pub provider: Option<String>,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemOneErrorKind {
    /// Malformed request or a question that failed validation (400, 422, 413).
    InvalidRequest,
    /// Missing or bad key (401, 403).
    Unauthorized,
    /// Out of credits (402) — gateway only.
    InsufficientCredits,
    RateLimited,
    /// Temporarily overloaded (529) or a 5xx.
    Overloaded,
    Other,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct SystemOneError {
    pub message: String,
    pub kind: SystemOneErrorKind,
    /// Seconds to wait before retrying, when the server said.
    pub retry_after_seconds: Option<u32>,
}

// ── Contract ──────────────────────────────────────────────────────────────────

/// Fast, typed, probabilistic decisions about a piece of state.
pub trait SystemOne: Send + Sync {
    fn decide<'a>(
        &'a self,
        request: SystemOneRequest,
    ) -> BoxFuture<'a, Result<SystemOneResponse, SystemOneError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

const NONE_MESSAGE: &str = concat!(
    r#"systemOne = "none": no System One vendor is selected. "#,
    r#"Set `[adapters] systemOne = "openrouter"` in fiducial.toml, "#,
    "then re-run `fid derive`.",
);

/// No-op System One — every call returns `SystemOneError` naming the fix.
pub struct NoneSystemOne;

impl SystemOne for NoneSystemOne {
    fn decide<'a>(
        &'a self,
        _request: SystemOneRequest,
    ) -> BoxFuture<'a, Result<SystemOneResponse, SystemOneError>> {
        Box::pin(async {
            Err(SystemOneError {
                message: NONE_MESSAGE.to_owned(),
                kind: SystemOneErrorKind::Unauthorized,
                retry_after_seconds: None,
            })
        })
    }
}

//! Diagnostics contract — error tracking and observability.
//!
//! Maps to `errors` in `fiducial.toml [adapters]`. Named `Diagnostics` in
//! Rust to avoid the awkward `ErrorsError` type that `errors.rs` would produce.
//!
//! Intentionally **synchronous**: capture-and-send is fire-and-forget; the
//! caller must not block on it. Vendors that send over the network do so in a
//! background queue, not in the call that captured the error.

/// Severity of a captured message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Debug => f.write_str("debug"),
            Level::Info => f.write_str("info"),
            Level::Warning => f.write_str("warning"),
            Level::Error => f.write_str("error"),
            Level::Fatal => f.write_str("fatal"),
        }
    }
}

/// An error returned by a [`Diagnostics`] method — in practice, only the
/// `None` implementation would produce one, and it never does.
#[derive(Debug, thiserror::Error)]
#[error("diagnostics error: {0}")]
pub struct DiagnosticsError(pub String);

/// Error tracking and observability contract.
///
/// Implementations must be `Send + Sync` — they will be wrapped in `Arc<dyn
/// Diagnostics>` and shared across threads and async tasks.
pub trait Diagnostics: Send + Sync {
    /// Capture an error with optional additional context.
    fn capture_error(&self, error: &dyn std::error::Error, context: &str);

    /// Capture a free-form message at the given level.
    fn capture_message(&self, level: Level, message: &str);
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op diagnostics — all captures are silently dropped.
///
/// `errors = "none"` is the default: the contract is wired in from day one and
/// costs nothing until pointed at a vendor. Switching to Sentry or Workers
/// Analytics is a config change, not a refactor.
pub struct NoneDiagnostics;

impl Diagnostics for NoneDiagnostics {
    fn capture_error(&self, _error: &dyn std::error::Error, _context: &str) {}
    fn capture_message(&self, _level: Level, _message: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> Box<dyn Diagnostics> {
        Box::new(NoneDiagnostics)
    }

    #[test]
    fn capture_error_is_silent() {
        let err = std::io::Error::other("boom");
        d().capture_error(&err, "during startup");
    }

    #[test]
    fn capture_message_is_silent() {
        d().capture_message(Level::Warning, "disk usage above 80%");
    }

    #[test]
    fn none_diagnostics_is_object_safe() {
        let _: Box<dyn Diagnostics> = Box::new(NoneDiagnostics);
    }

    #[test]
    fn level_display() {
        assert_eq!(Level::Error.to_string(), "error");
        assert_eq!(Level::Fatal.to_string(), "fatal");
    }
}

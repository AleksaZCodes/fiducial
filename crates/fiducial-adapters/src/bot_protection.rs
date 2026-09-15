//! Bot protection contract — verify a challenge token from a form submission.
//!
//! Designed against the narrowest interface shared by Turnstile, reCAPTCHA
//! and hCaptcha: all three are "the client solved a widget, hand me the
//! token, tell me if it was real." Vendor-specific extras (reCAPTCHA v3's
//! score, hCaptcha's enterprise risk signals) stay on the vendor type.
//!
//! **Only `none` ships a Rust implementation.** `turnstile` is real, but only
//! on the TypeScript side (`packages/adapters/src/bot-protection.ts`): every
//! product shape in this repository that submits a form — a Next.js/SvelteKit
//! server action, a Cloudflare Worker — verifies the token from TypeScript,
//! not from a Tauri desktop backend. A Rust implementation is not blocked the
//! way `d1`/`r2` are (this is a plain HTTPS call, reachable from anywhere);
//! it is simply unbuilt for lack of a Rust-reachable consumer. See
//! `docs/specs/2026-09-15-turnstile-and-queues.md`.

use thiserror::Error;

use crate::BoxFuture;

/// What the verification provider reported back.
///
/// Deliberately narrow: `success`, `challenge_ts` and `hostname` are the
/// fields Turnstile, reCAPTCHA and hCaptcha all return. A score exists on
/// some but not all of them, so it is not part of this contract.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifyOutcome {
    /// Whether the token was valid and unexpired.
    pub success: bool,
    /// Provider-reported timestamp of the challenge solve, ISO 8601.
    pub challenge_ts: Option<String>,
    /// The hostname the widget was served from, as the provider saw it.
    pub hostname: Option<String>,
}

/// Errors a [`BotProtection`] implementation may return.
#[derive(Debug, Error)]
pub enum BotProtectionError {
    #[error("verification request failed: {0}")]
    Request(String),
    #[error("{0}")]
    Other(String),
}

/// Bot / abuse challenge verification contract.
pub trait BotProtection: Send + Sync {
    /// Verify a client-solved challenge token.
    ///
    /// `remote_ip` is optional context some providers use to strengthen the
    /// check; omitting it must never be treated as a failure.
    fn verify<'a>(
        &'a self,
        token: &'a str,
        remote_ip: Option<&'a str>,
    ) -> BoxFuture<'a, Result<VerifyOutcome, BotProtectionError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op bot protection — **fails open**: every token verifies as `success`.
///
/// This is not a placeholder in the sense the other `None*` types are; it is
/// a real, working choice with a sharp edge stated plainly: `botProtection =
/// "none"` means the product has **no bot protection at all**, not "protection
/// pending." Choosing it is choosing to accept the traffic. That is the
/// correct default cost — a product with no adversary yet should not be
/// blocked by an unconfigured challenge widget — but it must never be mistaken
/// for "protected."
pub struct NoneBotProtection;

impl BotProtection for NoneBotProtection {
    fn verify<'a>(
        &'a self,
        _token: &'a str,
        _remote_ip: Option<&'a str>,
    ) -> BoxFuture<'a, Result<VerifyOutcome, BotProtectionError>> {
        Box::pin(std::future::ready(Ok(VerifyOutcome {
            success: true,
            challenge_ts: None,
            hostname: None,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bp() -> Box<dyn BotProtection> {
        Box::new(NoneBotProtection)
    }

    #[tokio::test]
    async fn verify_always_succeeds() {
        let outcome = bp().verify("any-token", None).await.unwrap();
        assert!(outcome.success);
    }

    #[tokio::test]
    async fn verify_ignores_remote_ip() {
        let outcome = bp().verify("any-token", Some("203.0.113.1")).await.unwrap();
        assert!(outcome.success);
    }

    #[test]
    fn none_bot_protection_is_object_safe() {
        let _: Box<dyn BotProtection> = Box::new(NoneBotProtection);
    }
}

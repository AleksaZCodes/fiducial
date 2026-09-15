//! Auth contract — users and authentication: sign-up, sign-in, sessions.
//!
//! Designed against the narrowest interface shared by Supabase Auth, Clerk
//! and Auth.js: email/password sign-up and sign-in, OAuth via redirect,
//! sign-out, password reset, and reading the current session. Vendor-specific
//! extras (Supabase row-level security, Clerk's organizations, Auth.js
//! adapters) stay on the vendor.
//!
//! **Only `none` ships a Rust implementation.** `supabase` is real, but only
//! on the TypeScript side (`packages/adapters/src/auth.ts`) — auth in every
//! product shape here happens from a Next.js/SvelteKit server action or a
//! Worker, the same TypeScript-only reasoning `turnstile` has (see
//! `bot_protection.rs`), and for the same non-structural cause: nothing
//! about verifying a session is Worker-binding-only, there is simply no
//! Rust-reachable consumer yet. See
//! `docs/specs/2026-09-15-auth-contract.md`.
//!
//! ## Why there is no session-store parameter here
//!
//! The TypeScript `Auth` interface is deliberately request-scoped: a real
//! implementation needs to read and write session material somewhere
//! (an httpOnly cookie for server-rendered apps, or nothing at all for a
//! bearer client that owns its own token) between calls, and that storage
//! shape is inherently a web-framework concern — `Request`/`Response`,
//! cookies, headers. Modeling it in Rust here, with no consumer to validate
//! the design against, would be exactly the speculative-contract mistake
//! *capability taxonomy, made real* already corrected once. The operation
//! signatures below are complete and real; the storage abstraction is not
//! part of this trait.

use thiserror::Error;

use crate::BoxFuture;

/// The signed-in principal.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    /// ISO 8601.
    pub created_at: String,
}

/// A live session: the tokens plus the user they belong to.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: i64,
    pub user: AuthUser,
}

/// Errors an [`Auth`] implementation may return.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("email not confirmed")]
    EmailNotConfirmed,
    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("{0}")]
    Other(String),
}

/// Users and authentication contract.
pub trait Auth: Send + Sync {
    fn sign_up<'a>(
        &'a self,
        email: &'a str,
        password: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>>;

    fn sign_in<'a>(
        &'a self,
        email: &'a str,
        password: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>>;

    fn sign_out<'a>(&'a self) -> BoxFuture<'a, Result<(), AuthError>>;

    /// The current session, if any — verified and refreshed as needed.
    fn get_session<'a>(&'a self) -> BoxFuture<'a, Result<Option<AuthSession>, AuthError>>;

    /// Send a password-reset email. `redirect_to` is where the reset link
    /// sends the user after they click it.
    fn reset_password_for_email<'a>(
        &'a self,
        email: &'a str,
        redirect_to: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthError>>;

    /// Set a new password for the currently authenticated user.
    fn update_password<'a>(&'a self, new_password: &'a str)
        -> BoxFuture<'a, Result<(), AuthError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op auth — every write fails with [`AuthError::Other`], `get_session`
/// always reports signed-out.
///
/// Unlike most `None*` types, this is not a silently-succeeding no-op: a
/// product that has not chosen an auth vendor should fail loudly if it tries
/// to sign someone in, not pretend to succeed and hand back a fabricated
/// session. `auth = "none"` means "no auth wired up," not "auth that always
/// works."
pub struct NoneAuth;

impl Auth for NoneAuth {
    fn sign_up<'a>(
        &'a self,
        _email: &'a str,
        _password: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(
            "auth = \"none\" — no auth vendor configured".into(),
        ))))
    }

    fn sign_in<'a>(
        &'a self,
        _email: &'a str,
        _password: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(
            "auth = \"none\" — no auth vendor configured".into(),
        ))))
    }

    fn sign_out<'a>(&'a self) -> BoxFuture<'a, Result<(), AuthError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn get_session<'a>(&'a self) -> BoxFuture<'a, Result<Option<AuthSession>, AuthError>> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn reset_password_for_email<'a>(
        &'a self,
        _email: &'a str,
        _redirect_to: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(
            "auth = \"none\" — no auth vendor configured".into(),
        ))))
    }

    fn update_password<'a>(
        &'a self,
        _new_password: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(
            "auth = \"none\" — no auth vendor configured".into(),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> Box<dyn Auth> {
        Box::new(NoneAuth)
    }

    #[tokio::test]
    async fn sign_up_fails_loudly() {
        assert!(auth().sign_up("a@example.com", "pw").await.is_err());
    }

    #[tokio::test]
    async fn sign_in_fails_loudly() {
        assert!(auth().sign_in("a@example.com", "pw").await.is_err());
    }

    #[tokio::test]
    async fn sign_out_succeeds_silently() {
        auth().sign_out().await.unwrap();
    }

    #[tokio::test]
    async fn get_session_reports_signed_out() {
        assert!(auth().get_session().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reset_password_fails_loudly() {
        assert!(auth()
            .reset_password_for_email("a@example.com", "https://example.com")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn update_password_fails_loudly() {
        assert!(auth().update_password("new-password").await.is_err());
    }

    #[test]
    fn none_auth_is_object_safe() {
        let _: Box<dyn Auth> = Box::new(NoneAuth);
    }
}

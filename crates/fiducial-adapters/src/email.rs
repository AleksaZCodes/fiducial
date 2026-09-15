//! Email contract — transactional email: send, template, domain verification.
//!
//! Designed against the narrowest interface shared by Resend, SES and
//! Cloudflare Email Routing. Vendor-specific features (Resend batch sends,
//! SES configuration sets, Cloudflare's SPF/DKIM helpers) stay on the vendor.

use thiserror::Error;

use crate::BoxFuture;

/// An outbound email message.
#[derive(Debug, Clone)]
pub struct Message {
    /// `name <addr>` or bare `addr`.
    pub from: String,
    /// One or more recipient addresses.
    pub to: Vec<String>,
    pub subject: String,
    /// HTML body (required). Plain-text alt is derived by stripping tags if
    /// the vendor supports it; provide `text` explicitly to control it.
    pub html: String,
    /// Optional plain-text alternative.
    pub text: Option<String>,
    /// Optional reply-to address.
    pub reply_to: Option<String>,
}

/// Errors an [`Email`] implementation may return.
#[derive(Debug, Error)]
pub enum EmailError {
    #[error("send rejected by provider: {reason}")]
    Rejected { reason: String },
    #[error("invalid address: {addr}")]
    InvalidAddress { addr: String },
    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("{0}")]
    Other(String),
}

/// Transactional email contract.
pub trait Email: Send + Sync {
    /// Send a single transactional message.
    ///
    /// Returns the provider-assigned message ID on success.
    fn send<'a>(&'a self, message: &'a Message) -> BoxFuture<'a, Result<String, EmailError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op email sender — all sends succeed silently, returning an empty ID.
///
/// Useful in development and testing: the product is wired for email from the
/// first line, and switching to Resend in production is a config change, not a
/// refactor.
pub struct NoneEmail;

impl Email for NoneEmail {
    fn send<'a>(&'a self, _message: &'a Message) -> BoxFuture<'a, Result<String, EmailError>> {
        Box::pin(std::future::ready(Ok(String::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email() -> Box<dyn Email> {
        Box::new(NoneEmail)
    }

    fn msg() -> Message {
        Message {
            from: "hello@example.com".into(),
            to: vec!["user@example.com".into()],
            subject: "Test".into(),
            html: "<p>Hi</p>".into(),
            text: None,
            reply_to: None,
        }
    }

    #[tokio::test]
    async fn send_succeeds_silently() {
        let id = email().send(&msg()).await.unwrap();
        assert_eq!(id, "");
    }

    #[test]
    fn none_email_is_object_safe() {
        let _: Box<dyn Email> = Box::new(NoneEmail);
    }
}

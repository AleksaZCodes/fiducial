//! Newsletter contract — subscriber list management.
//!
//! Designed against the intersection shared by Resend, Buttondown, Loops,
//! Listmonk and Mailchimp: subscribe an address, mark it unsubscribed (never
//! delete — the opt-out record is a suppression entry), and retrieve its
//! current status.
//!
//! **Only `none` ships a Rust implementation.** `resend` is real, but only on
//! the TypeScript side (`packages/adapters/src/newsletter.ts`): every product
//! shape in this repository that renders a subscribe form — a Next.js/SvelteKit
//! server action, a Cloudflare Worker — calls the Resend Contacts API from
//! TypeScript. A Rust implementation is not blocked (this is a plain HTTPS
//! call, reachable from anywhere); it is unbuilt for lack of a Rust-reachable
//! consumer. See `docs/specs/2026-09-16-resend-and-the-newsletter-contract.md`.

use thiserror::Error;

use crate::BoxFuture;

/// Optional attributes carried by a new subscriber.
#[derive(Debug, Clone, Default)]
pub struct SubscriberAttributes {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// The canonical subscriber record returned by every method.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    /// Provider-assigned contact ID. Empty string when the adapter is `none`.
    pub id: String,
    pub email: String,
    /// `true` while the address is actively subscribed; `false` after unsubscribe.
    pub subscribed: bool,
}

/// Errors a [`Newsletter`] implementation may return.
#[derive(Debug, Error)]
pub enum NewsletterError {
    #[error("request failed: {0}")]
    Request(String),
    #[error("{0}")]
    Other(String),
}

/// Subscriber list management contract.
pub trait Newsletter: Send + Sync {
    /// Add `email` to the list, or re-subscribe it if it was previously
    /// unsubscribed. Idempotent: calling this for an already-subscribed address
    /// must not fail and must return the current [`Subscription`].
    fn subscribe<'a>(
        &'a self,
        email: &'a str,
        attrs: Option<&'a SubscriberAttributes>,
    ) -> BoxFuture<'a, Result<Subscription, NewsletterError>>;

    /// Mark `email` as unsubscribed. Does not delete the record — the opt-out
    /// entry serves as a suppression list entry.
    fn unsubscribe<'a>(
        &'a self,
        email: &'a str,
    ) -> BoxFuture<'a, Result<(), NewsletterError>>;

    /// Return the current subscription record, or `None` if the address has
    /// never been on the list.
    fn status<'a>(
        &'a self,
        email: &'a str,
    ) -> BoxFuture<'a, Result<Option<Subscription>, NewsletterError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op newsletter — every call succeeds silently and no data is stored.
///
/// Discarding a subscription is safe (no address is leaked, no suppression
/// list is bypassed), so this has no sharp edge to label — contrast with
/// `NoneBotProtection`, which fails open and must document that loudly.
/// It matches `NoneQueue`'s posture.
pub struct NoneNewsletter;

impl Newsletter for NoneNewsletter {
    fn subscribe<'a>(
        &'a self,
        email: &'a str,
        _attrs: Option<&'a SubscriberAttributes>,
    ) -> BoxFuture<'a, Result<Subscription, NewsletterError>> {
        Box::pin(std::future::ready(Ok(Subscription {
            id: String::new(),
            email: email.to_owned(),
            subscribed: true,
        })))
    }

    fn unsubscribe<'a>(
        &'a self,
        _email: &'a str,
    ) -> BoxFuture<'a, Result<(), NewsletterError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn status<'a>(
        &'a self,
        _email: &'a str,
    ) -> BoxFuture<'a, Result<Option<Subscription>, NewsletterError>> {
        Box::pin(std::future::ready(Ok(None)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nl() -> Box<dyn Newsletter> {
        Box::new(NoneNewsletter)
    }

    #[tokio::test]
    async fn subscribe_returns_subscription() {
        let sub = nl().subscribe("a@example.com", None).await.unwrap();
        assert_eq!(sub.email, "a@example.com");
        assert!(sub.subscribed);
        assert_eq!(sub.id, "");
    }

    #[tokio::test]
    async fn subscribe_with_attrs() {
        let attrs = SubscriberAttributes {
            first_name: Some("Alice".into()),
            last_name: None,
        };
        let sub = nl()
            .subscribe("a@example.com", Some(&attrs))
            .await
            .unwrap();
        assert!(sub.subscribed);
    }

    #[tokio::test]
    async fn unsubscribe_succeeds_silently() {
        nl().unsubscribe("a@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn status_returns_none() {
        let s = nl().status("a@example.com").await.unwrap();
        assert!(s.is_none());
    }

    #[test]
    fn none_newsletter_is_object_safe() {
        let _: Box<dyn Newsletter> = Box::new(NoneNewsletter);
    }
}

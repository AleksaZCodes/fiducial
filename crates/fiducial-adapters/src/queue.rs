//! Queue contract — the producer side of an asynchronous job/message queue.
//!
//! Designed against the narrowest interface shared by Cloudflare Queues, SQS
//! and a Postgres-backed queue: "hand me a message, it gets delivered." A
//! consumer is deliberately **not** part of this contract — see below.
//!
//! **Only `none` ships a Rust implementation**, and for a structural reason
//! this time, not a "nobody has asked yet" one: `cloudflare-queues` — the one
//! real vendor — is reached through a Workers binding
//! (`env.QUEUE.send()`/`sendBatch()`), which exists only inside a Worker, the
//! same boundary `d1` and `r2` sit behind. See
//! `docs/specs/2026-09-15-turnstile-and-queues.md`.
//!
//! ## Why there is no `receive`/`consume` method
//!
//! Cloudflare Queues, like most managed queues fronting a serverless
//! compute model, deliver a consumer's messages by **invoking** it — a
//! Worker exports a `queue(batch, env)` handler that the platform calls with
//! a batch, rather than the consumer polling a `receive()` endpoint. A pull
//! API would misdescribe how delivery actually works and would not be
//! implementable against the one real vendor this contract has. Consuming a
//! queue is an entry point a product's own code exports, not a call an
//! adapter makes — the same shape of boundary `deploy` already draws around
//! `wrangler deploy`.

use thiserror::Error;

use crate::BoxFuture;

/// Errors a [`Queue`] implementation may return.
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("send failed: {0}")]
    Send(String),
    #[error("{0}")]
    Other(String),
}

/// Asynchronous queue contract — producer side only.
pub trait Queue: Send + Sync {
    /// Enqueue a single message body.
    fn send<'a>(&'a self, body: &'a [u8]) -> BoxFuture<'a, Result<(), QueueError>>;

    /// Enqueue several message bodies as one batch.
    ///
    /// Vendors that have no native batch primitive may send sequentially;
    /// the contract makes no atomicity promise across the batch (unlike
    /// [`crate::Database::batch`], which does, because every vendor behind
    /// that trait has a real transaction or batch primitive to use).
    fn send_batch<'a>(&'a self, bodies: &'a [&'a [u8]]) -> BoxFuture<'a, Result<(), QueueError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op queue — every send succeeds silently and the message is discarded.
///
/// `queue = "none"` is a real, working selection: a product not yet doing
/// background work compiles, runs, and is testable without a live queue.
pub struct NoneQueue;

impl Queue for NoneQueue {
    fn send<'a>(&'a self, _body: &'a [u8]) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn send_batch<'a>(&'a self, _bodies: &'a [&'a [u8]]) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(std::future::ready(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q() -> Box<dyn Queue> {
        Box::new(NoneQueue)
    }

    #[tokio::test]
    async fn send_succeeds_silently() {
        q().send(b"hello").await.unwrap();
    }

    #[tokio::test]
    async fn send_batch_succeeds_silently() {
        q().send_batch(&[b"a".as_slice(), b"b".as_slice()])
            .await
            .unwrap();
    }

    #[test]
    fn none_queue_is_object_safe() {
        let _: Box<dyn Queue> = Box::new(NoneQueue);
    }
}

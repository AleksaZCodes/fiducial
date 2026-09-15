//! Cross-platform adapter contracts for Fiducial.
//!
//! Each module defines:
//! - A Rust async trait — the interface every vendor must satisfy.
//! - A `None*` struct — the no-op that costs nothing until pointed somewhere.
//!
//! Async methods return `BoxFuture` (a type alias for
//! `Pin<Box<dyn Future<Output = T> + Send + '_>>`). This keeps the traits
//! object-safe (`Box<dyn Database>`) without requiring the `async-trait` macro.
//!
//! ## Contract → `fiducial.toml` key
//!
//! | Trait            | `[adapters]` key |
//! |---|---|
//! | `Database`       | `database`        |
//! | `Storage`        | `storage`         |
//! | `Email`          | `email`           |
//! | `Diagnostics`    | `errors`          |
//! | `BotProtection`  | `botProtection`   |
//! | `Queue`          | `queue`           |
//! | `Auth`           | `auth`            |
//! | (n/a)            | `deploy`          |
//!
//! `deploy` is a build-time/pipeline concern, not a runtime call — no trait.
//!
//! ## Firmware
//!
//! Cloud adapter contracts do not apply to `no_std` firmware targets.
//! Firmware uses `embedded-hal` / Embassy HAL traits for hardware I/O, and
//! `fiducial-ota` for OTA delivery. See [`firmware`] for the boundary statement.
//!
//! ## Adding a vendor
//!
//! 1. Implement the trait for your vendor struct — **if the vendor is
//!    reachable from Rust.** `d1`, `r2` and `cloudflare-queues` are not: a
//!    Cloudflare Workers binding only exists inside a Worker, so those
//!    vendors implement only the TypeScript side (`packages/adapters`) and
//!    this crate has no struct for them. `turnstile` is a plain HTTPS call
//!    and *is* reachable from Rust, but has no Rust-side implementation
//!    either — every product shape here that submits a form does so from
//!    TypeScript (a Next.js/SvelteKit server action or a Worker), so a Rust
//!    client would have no consumer. `supabase` (auth) is the same story: a
//!    plain HTTPS API, no Rust consumer yet. Add one here only when
//!    something in Rust can actually reach, and would actually use, the
//!    vendor.
//! 2. Add the vendor key to `CONTRACTS` in `fiducial-cli/src/adapter.rs`
//!    (move it from `candidates` to `implementations`).
//! 3. Update the `fid-adapters` executor in `derive.rs` to emit the import.
//!
//! Nothing else changes — the contract is fixed; only the factory wiring grows.

use std::{future::Future, pin::Pin};

pub mod auth;
pub mod bot_protection;
pub mod database;
pub mod diagnostics;
pub mod email;
pub mod firmware;
pub mod queue;
pub mod storage;

pub use auth::{Auth, AuthError, AuthSession, AuthUser, NoneAuth};
pub use bot_protection::{BotProtection, BotProtectionError, NoneBotProtection, VerifyOutcome};
pub use database::{Database, DatabaseError, NoneDatabase};
pub use diagnostics::{Diagnostics, DiagnosticsError, NoneDiagnostics};
pub use email::{Email, EmailError, NoneEmail};
pub use queue::{NoneQueue, Queue, QueueError};
pub use storage::{NoneStorage, Storage, StorageError};

/// Object-safe async return type used by every contract trait.
///
/// Defined once here so each module imports it from `crate` rather than
/// writing `Pin<Box<dyn Future<...>>>` in every method signature.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

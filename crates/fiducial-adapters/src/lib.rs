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
//! | Trait        | `[adapters]` key |
//! |---|---|
//! | `Database`   | `database`        |
//! | `Storage`    | `storage`         |
//! | `Email`      | `email`           |
//! | `Diagnostics`| `errors`          |
//! | (n/a)        | `deploy`          |
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
//!    reachable from Rust.** `d1` and `r2` are not: a Cloudflare Workers
//!    binding only exists inside a Worker, so those two vendors implement
//!    only the TypeScript side (`packages/adapters`) and this crate has no
//!    `D1Database`/`R2Storage` struct. Add one here only when something in
//!    Rust can actually reach the vendor.
//! 2. Add the vendor key to `CONTRACTS` in `fiducial-cli/src/adapter.rs`
//!    (move it from `candidates` to `implementations`).
//! 3. Update the `fid-adapters` executor in `derive.rs` to emit the import.
//!
//! Nothing else changes — the contract is fixed; only the factory wiring grows.

use std::{future::Future, pin::Pin};

pub mod database;
pub mod diagnostics;
pub mod email;
pub mod firmware;
pub mod storage;

pub use database::{Database, DatabaseError, NoneDatabase};
pub use diagnostics::{Diagnostics, DiagnosticsError, NoneDiagnostics};
pub use email::{Email, EmailError, NoneEmail};
pub use storage::{NoneStorage, Storage, StorageError};

/// Object-safe async return type used by every contract trait.
///
/// Defined once here so each module imports it from `crate` rather than
/// writing `Pin<Box<dyn Future<...>>>` in every method signature.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

# fiducial-adapters

**Cross-platform adapter contracts for Fiducial — database, storage, email, diagnostics.**

[![crates.io](https://img.shields.io/crates/v/fiducial-adapters.svg)](https://crates.io/crates/fiducial-adapters)

This crate defines the async trait contracts that back the `[adapters]` block in
`fiducial.toml`. Each contract ships a `None*` no-op implementation suitable for
tests and scaffolds. Real vendor implementations (Postgres/Neon, S3/Supabase,
Resend, Sentry) satisfy the same traits and are swapped in via `fid derive`.

The same contracts exist as TypeScript interfaces in `@fiducial/adapters`
(`packages/adapters/`). Both sides are kept in sync by design **for the
contract and the `none` no-op**. Cloudflare's binding-reached vendors — `d1`
(database) and `r2` (storage), the first two real implementations — exist
only on the TypeScript side: a D1 or R2 binding is only reachable inside a
Cloudflare Worker, so there is nothing for a Rust `Database`/`Storage`
implementation in this crate to bind to yet. See
`docs/specs/2026-09-15-cloudflare-adapter-set.md`.

## Contracts

| Contract      | Rust trait     | `[adapters]` key | No-op             |
|---------------|----------------|------------------|-------------------|
| Relational DB | `Database`     | `database`       | `NoneDatabase`    |
| Object store  | `Storage`      | `storage`        | `NoneStorage`     |
| Transactional email | `Email`  | `email`          | `NoneEmail`       |
| Error / event tracking | `Diagnostics` | `diagnostics` | `NoneDiagnostics` |

## Firmware boundary

Cloud adapter contracts **do not apply** to `no_std` firmware targets.
Firmware uses `embedded-hal` / Embassy HAL for hardware access and
`fiducial-ota` for over-the-air updates. See `src/firmware.rs` for the
full mapping.

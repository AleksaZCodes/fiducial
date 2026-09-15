# fiducial-adapters

**Cross-platform adapter contracts for Fiducial — database, storage, email, diagnostics, bot protection, queue, auth.**

[![crates.io](https://img.shields.io/crates/v/fiducial-adapters.svg)](https://crates.io/crates/fiducial-adapters)

This crate defines the async trait contracts that back the `[adapters]` block in
`fiducial.toml`. Each contract ships a `None*` no-op implementation suitable for
tests and scaffolds. Real vendor implementations (Postgres/Neon, S3/Supabase,
Resend, Sentry) satisfy the same traits and are swapped in via `fid derive`.

The same contracts exist as TypeScript interfaces in `@fiducial/adapters`
(`packages/adapters/`). Both sides are kept in sync by design **for the
contract and the `none` no-op**. The real vendors that shipped so far — `d1`
(database), `r2` (storage), `turnstile` (bot protection), `cloudflare-queues`
(queue), `supabase` (auth) — exist only on the TypeScript side. `d1`, `r2`
and `cloudflare-queues` are binding-reached: a Cloudflare Workers binding
only exists inside a Worker, so there is nothing for a Rust implementation
in this crate to bind to yet. `turnstile` and `supabase` are different —
both are plain HTTPS calls, reachable from Rust — but every product shape
here that submits a form or signs someone in does so from TypeScript, so a
Rust client has no consumer either. See
`docs/specs/2026-09-15-cloudflare-adapter-set.md`,
`docs/specs/2026-09-15-turnstile-and-queues.md` and
`docs/specs/2026-09-15-auth-contract.md`.

## Contracts

| Contract               | Rust trait      | `[adapters]` key | No-op                |
|-------------------------|-----------------|-------------------|-----------------------|
| Relational DB           | `Database`      | `database`        | `NoneDatabase`        |
| Object store            | `Storage`       | `storage`         | `NoneStorage`         |
| Transactional email     | `Email`         | `email`           | `NoneEmail`           |
| Error / event tracking  | `Diagnostics`   | `errors`          | `NoneDiagnostics`     |
| Bot / abuse challenge   | `BotProtection` | `botProtection`   | `NoneBotProtection`   |
| Async queue (producer)  | `Queue`         | `queue`           | `NoneQueue`           |
| Users & authentication  | `Auth`          | `auth`            | `NoneAuth`            |

## Firmware boundary

Cloud adapter contracts **do not apply** to `no_std` firmware targets.
Firmware uses `embedded-hal` / Embassy HAL for hardware access and
`fiducial-ota` for over-the-air updates. See `src/firmware.rs` for the
full mapping.

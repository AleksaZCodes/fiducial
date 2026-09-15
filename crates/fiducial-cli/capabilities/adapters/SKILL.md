# Adapters capability

> Installed by `fid add adapters`. Enables `fid derive` to generate
> `src/adapters.generated.ts` from `[adapters]` in `fiducial.toml`.

## The loop

1. **Declare** vendor choices in `fiducial.toml`:
   ```toml
   [adapters]
   database = "none"   # or d1, supabase, neon, postgres
   storage  = "none"   # or r2, s3, supabase-storage
   email    = "none"   # or resend, ses
   errors   = "none"   # or sentry, workers-analytics
   ```
2. **Derive**: `fid derive` → writes `src/adapters.generated.ts`
3. **Import** in application code:
   ```ts
   import { createAdapters } from "./adapters.generated.js";
   const adapters = createAdapters(env);  // env = Cloudflare env bindings, undefined otherwise
   ```
4. **Gate**: `fid derive --check` in CI — fails if the factory is stale

## The contracts

Each contract is defined twice, in lock-step:
- **Rust**: `crates/fiducial-adapters/src/<contract>.rs` — trait + `None*` impl
- **TypeScript**: `packages/adapters/src/<contract>.ts` — interface + `None*` class

| `[adapters]` key | Rust trait    | TypeScript interface | No-op              |
|---|---|---|---|
| `database`       | `Database`    | `Database`           | `NoneDatabase`     |
| `storage`        | `Storage`     | `Storage`            | `NoneStorage`      |
| `email`          | `Email`       | `Email`              | `NoneEmail`        |
| `errors`         | `Diagnostics` | `Diagnostics`        | `NoneDiagnostics`  |

`deploy` is a build-time/pipeline concern, not a runtime call — no trait exists for it.

## Vendors

All contracts currently implement only `none`. Planned vendors are listed as
`candidates` in `crates/fiducial-cli/src/adapter.rs`. Selecting a candidate
fails `fid doctor` with a clear message — it is an intention, not a promise.

When a vendor ships:
1. Its class moves from `candidates` to `implementations` in `adapter.rs`.
2. `vendor_ts_class_and_path` in `derive.rs` maps the key to the import.
3. Callers change nothing — they call `createAdapters(env)` and get the new vendor.

## Firmware

Cloud adapter contracts do not apply to `no_std` firmware targets.
Firmware uses `embedded-hal` / Embassy traits. See
`crates/fiducial-adapters/src/firmware.rs` for the boundary statement.

## Tests

```sh
cargo test -p fiducial-adapters        # unit tests for all 4 contracts
cargo test -p fiducial-cli adapters_   # end-to-end through the real binary
```

# Adapters capability

> Installed by `fid add adapters`. Enables `fid derive` to generate
> `src/adapters.generated.ts` from `[adapters]` in `fiducial.toml`.

## The loop

1. **Declare** vendor choices in `fiducial.toml`:
   ```toml
   [adapters]
   database = "none"   # or d1 (real), supabase, neon, postgres (planned)
   storage  = "none"   # or r2 (real), s3, supabase-storage (planned)
   email    = "none"   # or resend, ses (planned)
   errors   = "none"   # or sentry, workers-analytics (planned)
   ```
2. **Derive**: `fid derive` → writes `src/adapters.generated.ts`
3. **Import** in application code:
   ```ts
   import { createAdapters } from "./adapters.generated.js";
   const adapters = createAdapters(env);  // env = Cloudflare env bindings, undefined otherwise
   ```
4. **Gate**: `fid derive --check` in CI — fails if the factory is stale

## `d1` and `r2` — real, Workers-only

`database = "d1"` and `storage = "r2"` are real implementations, reached
through a Workers binding — they only work inside a `worker-cloudflare` app,
never from a Tauri desktop backend (D1/R2 have no client reachable outside
the Workers runtime; see `docs/specs/2026-09-15-cloudflare-adapter-set.md`).

Both read a **fixed binding name** off `env`, because `fid derive` cannot see
what name a hand-edited `wrangler.toml` chose:

```toml
# wrangler.toml
[[d1_databases]]
binding       = "DB"       # D1Database reads env.DB
database_name = "..."
database_id   = "..."

[[r2_buckets]]
binding     = "BUCKET"     # R2Storage reads env.BUCKET
bucket_name = "..."
```

Bind under a different name and either pass a shaped `env` (`{ DB: realBinding }`)
or wrap the class — `createAdapters(env)` still needs the object it constructs
each class with to carry the right property.

`R2Storage.signedUrl` throws rather than returning a URL: a presigned R2 URL
needs SigV4 signing against the S3-compatible API with an R2 API token, which
the binding does not carry. See the class doc comment in
`packages/adapters/src/storage.ts`.

## The contracts

Each contract is defined twice, in lock-step, for the **contract and the
`none` no-op** — a Cloudflare binding vendor (`d1`, `r2`) exists only on the
TypeScript side, because the binding itself only exists inside a Worker:
- **Rust**: `crates/fiducial-adapters/src/<contract>.rs` — trait + `None*` impl
- **TypeScript**: `packages/adapters/src/<contract>.ts` — interface + `None*` class (+ `D1Database`, `R2Storage`)

| `[adapters]` key | Rust trait    | TypeScript interface | No-op              |
|---|---|---|---|
| `database`       | `Database`    | `Database`           | `NoneDatabase`     |
| `storage`        | `Storage`     | `Storage`            | `NoneStorage`      |
| `email`          | `Email`       | `Email`              | `NoneEmail`        |
| `errors`         | `Diagnostics` | `Diagnostics`        | `NoneDiagnostics`  |

`deploy` is a build-time/pipeline concern, not a runtime call — no trait exists for it.

## Vendors

`database` and `storage` each have one real vendor now (`d1`, `r2`); every
other contract, and every other vendor on those two, still implements only
`none`. Planned vendors are listed as `candidates` in
`crates/fiducial-cli/src/adapter.rs`. Selecting a candidate fails `fid doctor`
with a clear message — it is an intention, not a promise.

When a vendor ships:
1. Its class moves from `candidates` to `implementations` in `adapter.rs`.
2. `vendor_ts_class_and_path` in `derive.rs` maps the key to the import.
3. Callers change nothing — they call `createAdapters(env)` and get the new vendor.

That is exactly what `d1` and `r2` did — no third step was invented for them.

## Firmware

Cloud adapter contracts do not apply to `no_std` firmware targets.
Firmware uses `embedded-hal` / Embassy traits. See
`crates/fiducial-adapters/src/firmware.rs` for the boundary statement.

## Tests

```sh
cargo test -p fiducial-adapters        # unit tests for all 4 contracts
cargo test -p fiducial-cli adapters_   # end-to-end through the real binary
```

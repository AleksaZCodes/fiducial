# Supabase database and storage adapters

**Date:** 2026-09-16  
**Status:** Shipped

## What this spec decides

1. The `Database` contract's `supabase`, `neon`, and `postgres` vendors are
   implemented — all three via direct Postgres connection, not PostgREST.
2. The `Storage` contract's `supabase-storage` vendor is implemented — via
   Supabase's Storage REST API.
3. `neon` and `postgres` are aliases for `SupabaseDatabase`, not separate
   classes. A migration between the three is a connection string swap.

## Decision: direct Postgres, not PostgREST

The `Database` contract is `execute` / `query` / `queryOne` / `batch` with
raw SQL and positional `SqlValue[]` params.

`supabase-js` speaks PostgREST (`.from().select()`), not SQL. Bridging it to
the `Database` contract has two honest options:

**Option A — `rpc()` escape hatch.** Wraps every statement in a Postgres
function. Requires a migration for every SQL shape a caller might use.
`batch` atomicity becomes the caller's problem — PostgREST has no transaction
spanning multiple requests.

**Option B — direct Postgres connection.** Speaks the Postgres wire protocol
directly. Any SQL, genuine `BEGIN`/`COMMIT` for `batch`, and the same driver
neon and postgres would use.

**Option B is the correct answer.** Option A satisfies the `Database`
interface's shape while breaking its semantics (`batch` without atomicity is
the "lock-in wearing a portability costume" the adapter module doc warns
about). Option B is honest.

## `batch` and atomicity

`batch` wraps all statements in a single transaction via `client.begin()`.
This satisfies the contract's atomicity guarantee and is the same primitive
`D1Database.batch()` uses (D1's batch API also runs atomically).

## The `PostgresClient` interface

`SupabaseDatabase` takes a `PostgresClient` rather than a connection string,
so `@fiducial/adapters` stays free of a native Postgres driver dependency.
The caller decides which driver to wire in:

```ts
import postgres from "postgres";
const sql = postgres(env.DATABASE_URL);
const db = new SupabaseDatabase({ client: sql });
```

The `PostgresClient` interface exports three methods: `unsafe(query, params)`,
`begin(fn)`, and `end()`. Any `postgres.js`-compatible driver satisfies it.

## `neon` and `postgres` come nearly free

The three vendors are the same wire protocol and the same driver. Rather than
three near-identical classes, `NeonDatabase` and `PostgresDatabase` are
exported aliases for `SupabaseDatabase`. Selecting any of the three generates
a different class name in the factory, which makes the declaration honest.

Moving between the three vendors is a `DATABASE_URL` change, not a code change.

## `SupabaseStorage`

Supabase Storage exposes a REST API at `{SUPABASE_URL}/storage/v1/object/…`.
All five `Storage` methods map to REST endpoints:

| Method | Endpoint | Notes |
|---|---|---|
| `put` | `POST /object/{bucket}/{key}` | `x-upsert: true` |
| `get` | `GET /object/{bucket}/{key}` | 404 → null |
| `delete` | `DELETE /object/{bucket}` | body: `{prefixes: [key]}` |
| `list` | `POST /object/list/{bucket}` | paginated (100/page) |
| `signedUrl` | `POST /object/sign/{bucket}/{key}` | `{expiresIn: ttlSeconds}` |

`signedUrl` is fully implemented — unlike `R2Storage`, which requires SigV4
signing outside the Workers runtime.

**Authentication:** uses `SUPABASE_SERVICE_ROLE_KEY` (bypasses RLS, correct
for server-side storage). The anon key is insufficient for write operations
in a default Supabase project.

## Secret convention

| Adapter | Secrets |
|---|---|
| `database = "supabase"` / `"neon"` / `"postgres"` | `DATABASE_URL` |
| `storage = "supabase-storage"` | `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY` |
| `storage = "supabase-storage"` (optional) | `SUPABASE_STORAGE_BUCKET` (default: `"assets"`) |

When a product selects both `auth = "supabase"` and `database = "supabase"`,
it holds two connections to the same project: the auth adapter uses
`supabase-js`'s GoTrue client; the database adapter uses a direct Postgres
connection. This is fine — they serve different purposes.

## The mechanical seam

| Where | Change |
|---|---|
| `adapter::CONTRACTS` | `database`: `supabase`, `neon`, `postgres` move from `candidates` to `implementations`. `storage`: `supabase-storage` moves from `candidates` to `implementations`. |
| `vendor_ts_class_and_path` in `derive.rs` | Three arms for `database/supabase|neon|postgres`; one for `storage/supabase-storage` |
| `run_fid_deploy` secrets | `DATABASE_URL` for Postgres vendors; `SUPABASE_URL` + `SUPABASE_SERVICE_ROLE_KEY` for supabase-storage |
| `packages/adapters/src/database.ts` | `PostgresClient` interface, `SupabaseDatabase`, `NeonDatabase` alias, `PostgresDatabase` alias |
| `packages/adapters/src/storage.ts` | `SupabaseStorage` |
| `packages/adapters/src/index.ts` | New exports |
| `crates/fiducial-cli/tests/adapters_pipeline.rs` | 5 new tests, 1 updated |

## Rust side

`SupabaseDatabase` and `SupabaseStorage` are TypeScript-only for two different
reasons:

- `SupabaseStorage` — **not structural, no consumer.** Supabase's Storage REST
  API is reachable from Rust, but nothing in this repository calls it.
- `SupabaseDatabase` — **structural.** A Postgres driver for a Rust `no_std`
  target does not exist. The Rust side already has `NoneDatabase` and the
  `Database` trait; that is all a Rust consumer needs today.

The Rust `fiducial-adapters` crate already exports `NoneDatabase` and
`NoneStorage`. No changes needed there.

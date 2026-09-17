# Supabase adapter set: `database = "supabase"` and `storage = "supabase-storage"`

**Date:** 2026-09-17
**Status:** accepted

---

## Context

`ROADMAP.md §"Supabase, fully"` records the request: *"I also need supabase to
be fully supported, including database, storage, auth and all those nice
things."* Auth was already shipped (`SupabaseAuth`, full cookie and bearer
flows). Postgres SQL dialect and RLS policy generation were already shipped.
This spec covers the two remaining pieces: a database adapter and a storage
adapter.

---

## `storage = "supabase-storage"` — `SupabaseStorage`

### Credential shape

| Env var | Source | Purpose |
|---|---|---|
| `SUPABASE_URL` | Project Settings → API | Endpoint for the JS client |
| `SUPABASE_SERVICE_ROLE_KEY` | Project Settings → API | Bypasses RLS for server-side writes |
| `SUPABASE_STORAGE_BUCKET` | Product env (optional) | Bucket name, defaults to `"assets"` |

`SUPABASE_URL` deduplicates with `auth = "supabase"` in the wrangler secrets
comment. A product using both auth and storage holds one URL and two keys.

### Client

`@supabase/supabase-js` (already a dependency for auth). The client is
initialised with `persistSession: false, autoRefreshToken: false` — server-side
only, no browser session plumbing.

### `list` semantics

Supabase Storage's `list()` is non-recursive: it returns the contents of one
folder at a time. The adapter provides BFS traversal: items with `id === null`
are implicit folder nodes; the adapter recurses into them until it reaches leaf
files. All returned keys are full paths from the bucket root.

### `signedUrl`

Fully implemented. Supabase Storage has a native `createSignedUrl` endpoint;
`ttlSeconds` is passed directly. This is the one capability R2Storage leaves
unimplemented (it would require SigV4 signing against a separate S3 API).

---

## `database = "supabase"` — `SupabaseDatabase`

### Why direct Postgres, not PostgREST

The `Database` contract exposes raw SQL: `execute`, `query`, `queryOne`, and
`batch`. `supabase-js` wraps PostgREST — a REST API over Postgres that does not
accept raw SQL, and that has no cross-request transaction. The `batch` method
promises atomicity; PostgREST cannot satisfy it. A direct Postgres wire-protocol
connection does: `batch` uses `sql.begin(tx => …)`.

### Credential shape

| Env var | Source | Purpose |
|---|---|---|
| `SUPABASE_DB_URL` | Project Settings → Database → Connection string (URI) | Full Postgres connection string |

This is distinct from `SUPABASE_ANON_KEY`. A product using both `auth =
"supabase"` and `database = "supabase"` holds two credentials to the same
project, one for each channel (REST via JS client, Postgres wire via `postgres`).

### Client

`postgres` (postgres.js, `^3.4.9`). Connects lazily on first query. Options:
`ssl: "require"` (mandatory for Supabase; Cloudflare Workers enforce TLS on
outbound TCP), `max: 1` (serverless: one connection per Worker instance is
enough and avoids exhausting Supabase's connection pool).

### `neon` and `postgres` candidates

All three Postgres-wire vendors (`supabase`, `neon`, `postgres`) share the same
adapter shape. When the `neon` and `postgres` candidates are promoted, they will
be thin aliases of `SupabaseDatabase` that read `NEON_DB_URL` and `DATABASE_URL`
respectively, with no other change.

---

## Promotion in `adapter.rs`

`supabase` moved from `database.candidates` to `database.implementations`.
`supabase-storage` moved from `storage.candidates` to `storage.implementations`.
Both `fid doctor` and `fid derive` recognise them as selectable.

The adapter.rs test `a_planned_vendor_reads_differently_from_a_typo` was updated
to test `"neon"` (still a candidate) rather than `"supabase"` (now implemented).

---

## `fid derive` wiring

`vendor_ts_class_and_path` in `derive.rs` maps:

```
("database", "supabase")          → SupabaseDatabase  @fiducial/adapters/database
("storage",  "supabase-storage")  → SupabaseStorage   @fiducial/adapters/storage
```

`run_fid_deploy` appends three secrets:
- `database = "supabase"` → `SUPABASE_DB_URL`
- `storage = "supabase-storage"` → `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`

`SUPABASE_URL` deduplicates with the auth block's entry.

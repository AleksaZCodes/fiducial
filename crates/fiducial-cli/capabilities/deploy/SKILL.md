# Deploy capability

> Installed by `fid add deploy`. Generates `apps/worker/wrangler.toml` from
> `[adapters]` + `[deploy]` in `fiducial.toml`.

## Why this exists

`wrangler.toml`'s bindings were a fact declared twice. `[adapters] database =
"d1"` already says the product wants D1, and `D1Database` already reads a
fixed `env.DB` — so a hand-written `[[d1_databases]]` block with `binding =
"DB"` restates what the platform already knows.

It was restated *wrongly*: the `worker-cloudflare` template shipped commented
examples binding `MY_DB` and `MY_BUCKET`, while every adapter reads `DB` and
`BUCKET`. A product following the template's own example got a Worker that
threw on its first query.

## The split

| Derived from `[adapters]` | Declared in `[deploy]` |
|---|---|
| which binding blocks exist | the D1 database id (`wrangler d1 create`) |
| each binding's **name** (`DB`, `BUCKET`, `QUEUE`) | the R2 bucket name |
| which secrets to name in a comment | the queue name, the route, `compatibility_date` |

`[deploy]` holds what only your Cloudflare account knows. Everything implied
by a vendor selection is generated around it.

## The loop

```toml
# fiducial.toml
[adapters]
deploy   = "cloudflare"
database = "d1"
storage  = "r2"

[deploy]
compatibility_date = "2025-01-01"
d1_database_id     = "…"   # from `wrangler d1 create`
r2_bucket_name     = "…"   # from `wrangler r2 bucket create`
```

```sh
fid derive           # writes apps/worker/wrangler.toml
fid derive --check   # CI gate: fails if stale or hand-edited
wrangler deploy
```

Change `storage` to `none` and the `[[r2_buckets]]` block disappears on the
next derive. That is the point: one selection, one place.

## Secrets are named, never written

An adapter that needs a secret (`turnstile` → `TURNSTILE_SECRET_KEY`,
`supabase` auth → `SUPABASE_URL`, `SUPABASE_ANON_KEY`) gets a comment naming
it and the `wrangler secret put` line to run. Values never enter the
generated file — a file in the repository is the one place a secret must not
be.

## Validation

`fid derive` fails, naming the field, when:

- `compatibility_date` is missing — pin one; a date that moves changes
  runtime behaviour under a product that did not ask it to
- a `REPLACE_…` placeholder survived for a vendor the product **actually
  selected** (an id is only demanded when its vendor is in `[adapters]`)

## Note on `wrangler.toml`

`fid add deploy` takes ownership of `apps/worker/wrangler.toml`: it becomes a
derived artifact, tracked in `fiducial.lock` and regenerated on every
`fid derive`. Any hand edits there are lost. Put what you were editing into
`[deploy]` instead — if it does not fit, that is a gap in the declaration
worth reporting rather than working around.

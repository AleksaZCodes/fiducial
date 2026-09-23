# Deploy capability

> Installed by `fid add deploy`. Generates a wrangler config from
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

## What is being deployed — `shape`

Two things get deployed to Cloudflare and they are not the same artifact.

| `shape` | What it is | `main` | Config lives at |
|---|---|---|---|
| `worker` (default) | a standalone Worker at `apps/worker/` | `src/index.ts` — a source file this repo wrote | `apps/worker/wrangler.toml` |
| `sveltekit` | `apps/web` **is** the Worker, via `@sveltejs/adapter-cloudflare` | `.svelte-kit/cloudflare/_worker.js` — *build output* | the product root |

Before `shape` existed this capability could only describe the first one, so
installing it on a SvelteKit product wrote an `apps/worker/wrangler.toml`
describing nothing the product runs while the real config stayed untracked.
That is worse than not running it, and it is why one product declared
`[adapters] deploy = "cloudflare"` and deliberately did not install this
capability.

An unknown `shape` is an error naming the value, not a silent fall back to the
default — a typo would otherwise generate a perfectly valid config for the
wrong thing.

`main` is **not** seeded into `[deploy]`. It defaults to whatever the declared
shape implies, so changing `shape` moves the entrypoint with it. Declare it
only to override.

```toml
[deploy]
shape              = "sveltekit"
name               = "upoznaj-biznis"
compatibility_date = "2026-08-31"
compatibility_flags = ["nodejs_compat"]
observability      = true
vars               = { SPOTS_DEFAULT = "14" }

# Namespaces no `[adapters]` contract covers. The id is load-bearing: point a
# deploy at a different one and the data is silently gone rather than missing.
[[deploy.kv_namespaces]]
binding = "SPOTS"
id      = "a6a236c7ea344ddca04548dd4c90fb44"
```

```toml
# pipelines/deploy.toml
outputs = ["wrangler.jsonc"]
```

`assets_directory` (default `.svelte-kit/cloudflare`) and `assets_binding`
(default `ASSETS`) are there for a product that moved them; a product that has
not declares neither.

## TOML or JSONC — the output path decides

A path ending `.json` or `.jsonc` is written as JSONC, anything else as TOML.
Both are formats wrangler reads, and which one a product wants is not derivable
from anything else — a SvelteKit app scaffolded by `create-cloudflare` already
has a `wrangler.jsonc`, and two configs in two formats disagreeing is exactly
the failure this capability exists to end.

## `vars` are plain text, and only plain text

`[deploy] vars` is for non-secret configuration — a default count, a feature
name, an environment label. It lands in the generated config in the clear,
which is why it is the wrong home for anything you would not paste into a pull
request. Those are secrets; see below.

`FIDUCIAL_PRODUCT` is always emitted, naming the product a running Worker
belongs to.

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

## Note on the generated config

`fid add deploy` takes ownership of the path in `pipelines/deploy.toml`
`outputs`: it becomes a derived artifact, tracked in `fiducial.lock` and regenerated on every
`fid derive`. Any hand edits there are lost. Put what you were editing into
`[deploy]` instead — if it does not fit, that is a gap in the declaration
worth reporting rather than working around.

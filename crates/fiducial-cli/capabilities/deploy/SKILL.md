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

## How the deploy runs — prefer Workers Builds

`wrangler deploy` from a laptop is the escape hatch, not the mechanism. It needs
a credential wherever it runs, and a deploy whose inputs are one machine's
environment is a deploy nobody can review or reproduce — principle 4.

**[Workers Builds](https://developers.cloudflare.com/workers/ci-cd/builds/) is
the default to reach for.** Cloudflare's own CI: connect the repository to the
Worker and a push to the production branch builds and deploys it. The reason it
wins is not convenience, it is that there is no credential to place anywhere —
from its configuration docs:

> By default, Cloudflare will automatically generate an API token for your
> account when using Workers Builds, and continue to use this API token for all
> subsequent builds.

So no `CLOUDFLARE_API_TOKEN` in a CI secret store, in a container, or on a
laptop. It also gives Preview URLs for non-production branches and build status
as pull-request comments. The free plan allows 3,000 build minutes a month and
one concurrent build; a web product of this shape builds in well under a minute.

Settings, for a product of this layout:

| Setting | Value |
|---|---|
| Root directory | `apps/web` (the shape's Worker; blank for `shape = "worker"`) |
| Build command | `pnpm install && pnpm run build` |
| Deploy command | `npx wrangler deploy` |

**The dashboard's Worker name must equal `name` in the wrangler config at that
root directory, or the build fails.** Since `name` is derived from the product
name, that is what to type.

### What this costs, honestly

Connecting the repository is **dashboard-only** — every documented path is
Settings → Builds → Connect, and there is no wrangler command, API call or
config field for it. So the connection itself is vendor-dashboard state, which
is exactly what principle 4 objects to, and this capability cannot derive it or
check it. What it can keep in the repository is everything the build *runs*: the
wrangler config, the scripts, the lockfile. That is the trade — one unreviewable
click, in exchange for no credential anywhere.

Reach for `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the environment
when Workers Builds cannot be used: a self-hosted Git provider, a provider it
does not support, or a deploy that must run from a sandbox. Wrangler reads both
without a `wrangler login`. Such an environment also needs egress to
`api.cloudflare.com`, and `WRANGLER_SEND_METRICS=false` — a blocked
`sparrow.cloudflare.com` (telemetry only) reports as the same `fetch failed` as a
real failure and sends you after the wrong thing.

## What is being deployed — `shape`

Three things get deployed to Cloudflare and they are not the same artifact.

| `shape` | What it is | `main` | Config lives at |
|---|---|---|---|
| `worker` (default) | a standalone Worker at `apps/worker/` | `src/index.ts` — a source file this repo wrote | `apps/worker/wrangler.toml` |
| `sveltekit` | `apps/web` **is** the Worker, via `@sveltejs/adapter-cloudflare` | `.svelte-kit/cloudflare/_worker.js` — *build output* | the product root |
| `nextjs` | `apps/web` **is** the Worker, via `@opennextjs/cloudflare` | `.open-next/worker.js` — *build output* | `apps/web/wrangler.jsonc` |

### `nextjs`

Set `shape = "nextjs"` and point `pipelines/deploy.toml`'s `outputs` at
`apps/web/wrangler.jsonc`. Assets default to `.open-next/assets` bound as
`ASSETS`, and the name defaults to the product name, as for `sveltekit`.

The capability derives the wrangler config and nothing else. The product still
adds, in `apps/web`:

```sh
pnpm add @opennextjs/cloudflare
pnpm add -D wrangler
```

an `open-next.config.ts` containing `defineCloudflareConfig()` from
`@opennextjs/cloudflare`, and two scripts:

```json
"preview": "opennextjs-cloudflare build && opennextjs-cloudflare preview",
"deploy":  "opennextjs-cloudflare build && opennextjs-cloudflare deploy"
```

**Run that second one as `pnpm run deploy`, never `pnpm deploy`.** `deploy` is
one of pnpm's own subcommands, so pnpm handles it itself and the script is never
reached:

```sh
pnpm -C apps/web deploy              # ERR_PNPM_NOTHING_TO_DEPLOY — builds nothing
pnpm --filter @p/web deploy          # ERR_PNPM_INVALID_DEPLOY_TARGET
pnpm -C apps/web run deploy          # the script
```

Both wrong forms exit in under a second with a message that reads as a
configuration complaint rather than "your script did not run", which is how
`aleksa.works` carried a documented deploy command that had never deployed
anything. The script keeps the name `deploy` — that is what Workers Builds and
every other runner expects — so it is the *invocation* that needs `run`.
`preview` is not a pnpm subcommand and needs nothing.

`.open-next/` and `.wrangler/` are build output and are gitignored in a new
product; ignore them in `biome.json` too, or the linter walks the bundle.

No `route` is written unless `[deploy] route` is declared, so a first deploy goes
to `workers.dev` and attaching the domain is a deliberate edit.

### A zone route and a custom domain are different things

`route` alone renders wrangler's **`route`**, a *zone* route: it matches traffic
already arriving at Cloudflare and creates no DNS record. The hostname must
already have a proxied record, and `wrangler deploy` will not make one — against
a registrar's parked apex it fails with `code: 100117`.

`custom_domain = true` renders the form that makes Cloudflare create the record
and issue the certificate itself:

```toml
[deploy]
route         = "aleksa.works"
custom_domain = true
workers_dev   = true
```

```jsonc
"routes": [{ "pattern": "aleksa.works", "custom_domain": true }],
"workers_dev": true
```

For a product whose Worker **is** the site — `sveltekit` and `nextjs` — the
custom domain is nearly always what is meant. It was inexpressible until this
field existed, so `aleksa.works` hand-edited its derived `wrangler.jsonc` to get
one, which is the single thing this capability exists to prevent.

`workers_dev` is declarable because **declaring any route flips wrangler's
default for it to off**, silently, taking down the URL a deploy gets checked
against. Undeclared, it is omitted rather than written `false` — saying nothing
and saying no are different.

The apex must be empty before the first custom-domain deploy. A registrar's
parking CNAME is enough to fail it.

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

# Classes this Worker exports and binds (SQLite-backed, as the free plan
# requires). `RealtimeDO` is `@fiducial/realtime`'s broadcast/presence room;
# re-export it from `main` (`export { RealtimeDO } from "@fiducial/realtime/durable-object"`).
[[deploy.durable_objects]]
binding = "REALTIME"
class   = "RealtimeDO"
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

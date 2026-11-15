# Skill: api-hono

**Capability:** `api-hono` · **Platform:** Fiducial {{version}}

---

## What this capability adds

A Hono API at `apps/api/`. Choose it when an app has no route handlers of its own
(an Astro site, a single-page app) or when the backend should stand apart from the
front end. Hono is built on Web standards, tiny, and the same app runs in three
places:

| Where | Entry |
|---|---|
| a Cloudflare Worker | `src/index.ts` (`export default app`) |
| Node, for the self-hosted target | `src/node.ts` (`pnpm start:node`) |
| a test, with no server at all | `app.request("/healthz")` in `src/app.test.ts` |

`src/app.ts` holds the routes and exports `AppType`, the route table as a type, so
a front end builds a typed client with `hc<AppType>()` and a request or response
that disagrees with the route is a compile error, not a runtime surprise.

## Declare it

```toml
[apps.api]
framework = "hono"

[deploy]
shape = "worker"
```

and derive the Worker config into this app:

```toml
# pipelines/deploy.toml
name     = "deploy"
executor = "fid-deploy"
outputs  = ["apps/api/wrangler.toml"]
```

`main` defaults to `src/index.ts`, relative to that file, so there is no entry to
keep in step by hand.

## Run it

```sh
pnpm install
cd apps/api
pnpm dev            # wrangler dev: the Worker, locally
pnpm dev:node       # the Node entry, watching
pnpm test           # node:test over app.request
pnpm typecheck
wrangler deploy
```

## On the self-hosted target

`deploy = "compose"` runs this app as a service named `api` and reads `/healthz`.
Declare it so Caddy proxies to it:

```toml
[deploy]
proxy      = "api"
proxy_port = 8080
```

## Add routes

Chain every route onto `app`: a route registered on its own line is not in
`AppType`, and the typed client will not see it.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

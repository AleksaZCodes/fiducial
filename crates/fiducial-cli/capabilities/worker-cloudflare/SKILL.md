# Skill: worker-cloudflare

**Capability:** `worker-cloudflare` · **Platform:** Fiducial {{version}}

---

## What this capability adds

A Cloudflare Worker (or Durable Object) at `apps/worker/`, using:

- **Wrangler** for local dev, deployment, and secret management
- **Miniflare** for fast local testing (no network required)
- **TypeScript** strict mode with `@cloudflare/workers-types`

## Development

```sh
pnpm dev --filter @{{name}}/worker    # local Wrangler dev server
pnpm test --filter @{{name}}/worker   # Miniflare test runner
pnpm --filter @{{name}}/worker wrangler deploy   # deploy to Cloudflare
```

## Secrets

```sh
# Set a secret in production
wrangler secret put MY_SECRET

# Set for a preview environment
wrangler secret put MY_SECRET --env preview
```

Never commit secrets. Declare them in `wrangler.toml` under `[vars]` (non-secret)
or reference them as `{ binding = "MY_SECRET" }` (secret, set via `wrangler secret`).

## Key constraints

- Do not use `fetch` in Worker code without pinning the URL or routing through a
  declared binding (KV, D1, R2, DO, Service).
- Durable Objects require a `wrangler deploy` for every schema migration — there
  is no rollback for storage. Design for forward compatibility.
- Test locally with Miniflare before deploying; `wrangler dev` also works but
  is slower for unit-level iteration.

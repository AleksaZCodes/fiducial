# A Next.js app on Cloudflare is a deploy shape, and the web scaffold's paths follow where Next looks

**Date:** 2026-11-05
**Status:** accepted

---

## Context

aleksa.works, a Next.js product, was taken to Cloudflare Workers on 2026-11-04.
Four things had to be done by hand that the platform should have said:

1. `[deploy] shape` knew `worker` and `sveltekit`. A Next.js product could only
   declare a standalone Worker whose `src/index.ts` it does not have, so its
   real `wrangler.jsonc` stayed untracked and hand-written. The same gap the
   `sveltekit` shape closed on 2026-09-15.
2. `pipelines/brand.toml` wrote `public/*` and `src/generated/brand.ts` at the
   product root. Next serves `apps/web/public/` and the app imports
   `apps/web/src/generated/`, and `seo`, `cms` and `render-og.mjs` already read
   `apps/web/src/generated/brand.ts`. The product moved all six paths before
   robots.txt, the favicon or the brand module reached anyone.
3. `.gitignore` did not cover `.open-next/`, `.wrangler/`, `*.tsbuildinfo` or
   `next-env.d.ts`, so a first build left untracked files.
4. `web-next`'s `tsconfig.json` differs from what `next build` rewrites it to,
   so every build makes a platform-owned file look hand-edited to `fid doctor`.
   Its `biome.json` did not ignore `public/` (derived, vendored fonts) or the
   OpenNext bundle, so `pnpm lint` failed or walked the bundle.

## Decision

- Add `shape = "nextjs"`. `apps/web` is the Worker through
  `@opennextjs/cloudflare`: `main = .open-next/worker.js`, assets
  `.open-next/assets` bound as `ASSETS`, name defaults to the product name, and
  no `FIDUCIAL_PRODUCT` var, the same reasoning as `sveltekit`.
- Move the brand pipeline's outputs under `apps/web/`, matching what its
  consumers already read.
- Ignore the new build directories in the scaffold's `.gitignore` and
  `biome.json`, and ship the `tsconfig.json` Next converges on.

## Why not the alternatives

- **Generate `open-next.config.ts` and the package scripts too.** The deploy
  pipeline has one output, and a capability cannot patch another file's
  `package.json` scripts. The skill documents the three manual steps instead.
- **Branch brand paths on whether `web-next` is installed.** One template with
  one answer is simpler, and `i18n` already made the same move to `apps/web/`.
- **Write a `routes` entry for the custom domain.** A custom-domain route makes
  `wrangler deploy` create the DNS record. That is a deliberate edit, so
  `[deploy] route` stays empty unless declared.

## Consequences

- Existing products that already point brand at `public/` keep working: the
  pipeline is product-owned, so `fid upgrade` does not move them.
- A new Next.js product gets a correct brand output, a clean first lint and a
  clean `fid doctor` after its first build.

## What this deliberately does not do

- It does not call Cloudflare. No token is read, and nothing is deployed.
- It does not make `fid upgrade` refresh the `fiducial-cli` version pinned in
  `ci.yml`. That is left alone on purpose, so version-stamped templates do not
  open spurious merges, and it means the pin goes stale until someone bumps it.
  Worth its own decision.

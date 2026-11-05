# The deploy belongs in Cloudflare's CI, and a custom domain is a declaration rather than a hand edit

**Date:** 2026-11-05
**Status:** accepted
**Supersedes:** nothing — extends `2026-11-05-next-on-cloudflare-is-a-deploy-shape.md`

---

## Context

`aleksa.works` went live on Workers today. Four things went wrong on the way,
and each of them was the platform's fault rather than the product's.

1. **A documented command that had never run.** The `deploy` capability's skill
   tells a product to add a `deploy` script, and the product's `ROADMAP.md`
   documented `pnpm -C apps/web deploy`. `deploy` is one of pnpm's own
   subcommands, so pnpm answered it itself:

   ```
   pnpm -C apps/web deploy           ERR_PNPM_NOTHING_TO_DEPLOY
   pnpm --filter @p/web deploy       ERR_PNPM_INVALID_DEPLOY_TARGET
   pnpm -C apps/web run deploy       the script
   ```

   Both wrong forms exit in under a second with what reads as a configuration
   complaint, not "your script did not run". The only reason it was caught is
   that someone ran the documented command and watched.

2. **A custom domain could not be declared.** `[deploy] route` renders
   wrangler's `route`, which is a *zone* route: it creates no DNS record. The
   apex still pointed at the registrar's parking page, so the deploy failed with
   `code: 100117`. The form that creates its own record and certificate —
   `routes: [{ pattern, custom_domain: true }]` — was inexpressible, so the
   product hand-edited the derived `wrangler.jsonc`. That is the one failure
   this capability exists to prevent, and the capability caused it.

3. **`workers.dev` went dark, silently.** Declaring any route flips wrangler's
   default for `workers_dev` to off. The first custom-domain deploy therefore
   took down the preview URL — the very URL the skill tells you to check a
   deploy against before trusting the domain — and reported success.

4. **CI gated with a `fid` that could not read the product.** `ci.yml` pinned
   `fiducial-cli` 0.9.4 against a config declaring `shape = "nextjs"`, which
   0.9.4 rejects by name. `2026-11-05-next-on-cloudflare-is-a-deploy-shape.md`
   left this open deliberately — "the pin goes stale until someone bumps it.
   Worth its own decision" — and this is that decision. The failure mode is
   worse than staleness: the error names the *product's* config as invalid, not
   the pin as old.

Separately, Cloudflare published a Claude Code setup guide and a Skills plugin
(`cloudflare/skills`) carrying ~20 platform skills and its MCP servers, and
`agent-setup/claude-code/` documents appending `/index.md` to any docs URL for
clean markdown. Reading that page was what settled item 2 above; not having it
is why the session before spent an afternoon at `curl` against the API.

## Decision

- **Workers Builds is the deploy the skill recommends**, and environment
  credentials are the escape hatch. The reason is not convenience: its docs
  state that Cloudflare generates and keeps the API token itself, so there is no
  credential to place in a secret store, a container or a laptop.
- **`[deploy] custom_domain`** switches `route` from a zone route to a
  custom-domain `routes` entry. Default `false`, so no existing product's
  generated config changes shape.
- **`[deploy] workers_dev`** is declarable, and *omitted* when undeclared —
  `None` is not `Some(false)`, because wrangler's default is not a constant.
- **`fid doctor` warns when `ci.yml`'s `fiducial-cli` pin is not this binary's
  version**, naming both and the line to edit.
- **The scaffolded `.claude/settings.json` enables `cloudflare@cloudflare`**
  alongside `fiducial@fiducial`, so every product gets the Cloudflare skills and
  MCP servers from the repository rather than from whatever each machine
  happened to install.

## Why not the alternatives

- **Rename the script from `deploy`.** It would close the trap by construction,
  but `deploy` is the name Workers Builds' own docs assume (`npm run deploy`)
  and every other runner expects. The invocation is what needs `run`; renaming
  trades a documented gotcha for an undocumented deviation.
- **Make `custom_domain` the default for `sveltekit` and `nextjs`.** It is what
  those shapes nearly always want. But it would silently change the generated
  config of every product already declaring a `route`, and a route changing
  shape under a deploy is how a site goes dark. A flag the product sets is one
  line; recovering from an unasked-for rewrite is not.
- **Write `workers_dev: false` when undeclared.** Then the generated file would
  state a default rather than a decision, and a product that wanted wrangler's
  behaviour would have to override the platform to get it.
- **Make the stale CI pin an issue rather than a warning.** An older pin is a
  legitimate state for a product deliberately held back, and a newer one is
  normal for the minutes after a release. Neither is a broken repository.
- **Have `fid upgrade` rewrite the pin.** Rejected again, for the reason the
  earlier spec gave: a version-stamped template opens spurious merges. Telling
  someone is the part that was missing, not doing it for them.
- **Keep deploying with `CLOUDFLARE_API_TOKEN` as the main path.** It works, and
  it is what every product does today. But it puts a credential in each
  environment that deploys, and the thing a token buys — a deploy from anywhere
  — is not something a product needs often enough to pay that with.

## Consequences

- A product can derive a complete wrangler config including its custom domain.
  `aleksa.works` can stop hand-writing `apps/web/wrangler.jsonc` and let the
  pipeline own it.
- `fid doctor` gains a warning that will fire on every existing product whose
  pin predates this release. That is correct and it is the point.
- The scaffold now depends on a marketplace this project does not control
  (`cloudflare/skills`). If Cloudflare moves or renames that plugin, new
  products get a settings file naming something that does not resolve. The cost
  of that is a Claude Code warning, not a broken build, which is why it is
  acceptable — but it is a real external dependency in a scaffold that had none.

## What this deliberately does not do

- **It does not connect anything to Cloudflare.** Workers Builds' repository
  connection is dashboard-only: there is no wrangler command, API endpoint or
  config field for it in the documentation. So the connection stays vendor state
  that this capability can neither derive nor check, and the skill says so
  rather than implying the repository describes the whole deploy.
- **It does not move Next.js off OpenNext.** Cloudflare now recommends `vinext`
  as the default way to run Next.js on Workers, and `vinext` is in beta.
  Migrating a live site onto a beta adapter to follow a recommendation is not a
  trade worth making today. Revisit when it leaves beta; the trigger is that,
  not a date.
- **It does not validate that `custom_domain`'s hostname is a zone on the
  account, or that the apex is empty.** `fid` makes no network calls. The apex
  must be clear before the first such deploy — a registrar's parking CNAME is
  enough to fail it — and that is documented, not enforced.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

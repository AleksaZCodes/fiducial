---
"fiducial-cli": minor
---

Deploy: a custom domain is declarable, and `fid doctor` notices a stale CI pin

`[deploy] custom_domain = true` switches `route` from a zone route — which
creates no DNS record and fails against a parked apex with `code: 100117` — to
the `routes` entry that makes Cloudflare create the record and certificate
itself. `[deploy] workers_dev` is declarable too, because declaring any route
flips wrangler's default for it to off and silently takes down the preview URL.
Both default to the previous behaviour, so no existing product's generated
config changes shape.

`fid doctor` now warns when `.github/workflows/ci.yml` pins a different
`fiducial-cli` than the running binary, naming both versions and the line to
edit. A stale pin made CI reject a product's own config as invalid rather than
report itself as old.

The `deploy` skill now recommends Workers Builds over environment credentials,
documents that `pnpm deploy` never reaches the `deploy` script (it is a pnpm
subcommand — use `pnpm run deploy`), and the scaffolded `.claude/settings.json`
enables Cloudflare's own Claude Code plugin alongside Fiducial's.

See `docs/specs/2026-11-05-the-deploy-is-cloudflares-ci-and-a-custom-domain-is-declarable.md`.

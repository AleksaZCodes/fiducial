---
"@fiducial/adapters": minor
---

`D1Database` and `R2Storage` — the first real vendor implementations of the
`database` and `storage` contracts, reached through Cloudflare Workers
bindings (`env.DB`, `env.BUCKET`).

`D1Database` wraps D1's `prepare().bind().run()/all()/first()` and
`batch()`. `R2Storage` wraps `put`/`get`/`delete`/`list` (following R2's
cursor across pages); `signedUrl` throws rather than silently returning an
empty string, naming why: a presigned R2 URL needs SigV4 signing against the
S3-compatible API with an R2 API token, which the binding does not carry.

Select with `database = "d1"` / `storage = "r2"` in `[adapters]` —
`fid derive` wires the real class in, no other change needed.

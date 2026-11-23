# Skill: payload

**Capability:** `payload` · **Platform:** Fiducial {{version}}

This skill is loaded by Claude Code automatically when the `payload` capability
is installed. Read it once at session start; do not re-derive it.

---

## What this capability adds

A [Payload CMS](https://payloadcms.com) 3 instance running inside the Next.js
app at `apps/web/`, persisted in D1 (`@payloadcms/db-d1-sqlite`) and storing
media in R2 (`@payloadcms/storage-r2`).

The admin UI lives at `/admin` on the same Worker as the site. Payload's built-in
auth is used — there is no customer collection and no public sign-up. Three roles
map onto the platform's identity model (D6 of `docs/specs/2026-11-22-payload-is-the-cms-and-commerce-capability.md`):

| Payload role | Principal | Resource | Role |
|---|---|---|---|
| `owner` | `{ kind: "user", id }` | `{ kind: "platform" }` | `owner` |
| `staff` | `{ kind: "user", id }` | `{ kind: "platform" }` | `member` |
| `agency-admin` (API key) | `{ kind: "service", id }` | `{ kind: "platform" }` | `viewer` |

## Development

```sh
pnpm dev --filter @{{name}}/web           # starts Next dev with Payload
pnpm --filter @{{name}}/web run payload   # payload CLI (generate:types etc.)
pnpm --filter @{{name}}/web run generate:types
pnpm --filter @{{name}}/web run generate:importmap
```

Payload's admin runs at `http://localhost:3000/admin`.

## The derive pipeline

`pipelines/payload.toml` runs `scripts/derive-payload.mjs`, which reads
`[i18n]` and `[brand]` from `fiducial.toml` and writes
`apps/web/src/generated/payload.ts`.

That module exports:

| Export | Is |
|---|---|
| `payloadLocales` | `[{ code, label }]` from `[i18n] locales` |
| `defaultLocale` | the `[i18n] default` locale |
| `adminLocale` | `"en"` — the closest Payload ships to `sr` |
| `entityName` | the `[brand] trading_name` |
| `PayloadRole` | `"owner" \| "staff" \| "agency-admin"` |

**Never hand-edit** `apps/web/src/generated/payload.ts`. Change `[i18n]` or
`[brand]` in `fiducial.toml` and run `fid derive`.

## The access module

`apps/web/src/payload/access.ts` exports three Payload access functions:
`isOwnerOrStaff`, `isOwner`, and `isAgencyAdmin`. They build an in-memory grant
from the Payload user document and call `can()` from `@fiducial/identity` — no
grants table is written to D1.

The `role` field on the Payload `users` collection is the single declaration of
who may do what.

## Collections

| Collection | Purpose |
|---|---|
| `users` | Staff and owner accounts — Payload auth, `role` field |
| `pages` | CMS pages with blocks (commit 4 adds the blocks) |
| `media` | Uploaded files, stored in R2 via `@payloadcms/storage-r2` |

The global `SiteSettings` stores site-wide defaults (site name, tagline,
default OG image).

## Superseded files

This capability supersedes two platform-owned templates:

| File | Why |
|---|---|
| `apps/web/next.config.ts` | Must be wrapped in `withPayload(config)` |
| `apps/web/open-next.config.ts` | Enables the R2 incremental ISR cache |

`fid doctor` accepts both because `supersedes` is declared in `capability.toml`.

## Configuration

`[deploy] secrets` must include `PAYLOAD_SECRET` (a random 32-char string) —
declared in `fiducial.toml`, placed with `wrangler secret put PAYLOAD_SECRET`.

The D1 binding (`DB`) and the R2 binding (`BUCKET`) are already derived from
`[adapters]` by the `deploy` capability, so nothing new is declared there.

## D1 and SQLite

Payload runs its own migrations with `payload migrate`. **Do not** install the
`migrations` capability alongside this one — D6 removes the only reason to.

Known D1 limitations (Payload issues #15219 and #14766): array-field updates
write without D1's `batch` API and SQLite transactions are a no-op. The
`commerce` capability is designed around this: no array fields in orders.

## Nightly backup

A Cron Trigger at `0 2 * * *` calls Cloudflare's D1 export REST endpoint and
writes the dump to `backup/<date>.sql.gz` in R2. Declared in `[deploy] crons`
(added by this capability's install) and `[deploy] secrets = ["BACKUP_TOKEN"]`.

**Restore procedure:** download from R2, run
`wrangler d1 execute <db> --file <dump>`. Record the last restore date in this
file as evidence.

## Worker limits

This capability requires the **Cloudflare Workers Paid plan** — Payload's
bundle exceeds the 1 MB free limit (Workers runs up to 64 MiB uncompressed
since September 2026).

## Rules

- Never put a value in `[deploy] secrets` — only the name.
- Never hand-edit `apps/web/src/generated/payload.ts`.
- Collections and globals are product-owned templates, so edit them freely.
- `payload generate:types` and `payload generate:importmap` are pipeline
  outputs — run them after any schema change, not by hand mid-session.

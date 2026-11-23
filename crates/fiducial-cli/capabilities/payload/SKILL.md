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
pnpm --filter @{{name}}/web run payload   # payload CLI
pnpm --filter @{{name}}/web run generate:types
pnpm --filter @{{name}}/web run generate:importmap
```

The `generate:types` and `generate:importmap` scripts use
`scripts/payload-ts-resolve.mjs` — a Node module hook that retries
extensionless relative imports with `.ts`, `.tsx`, and `/index.ts`.
This replaces the tsx-based path that double-evaluates undici's symbols
on Node ≥ 22, causing `TypeError: Illegal constructor`.

Payload's admin runs at `http://localhost:3000/admin`.

## The derive pipelines

`pipelines/payload.toml` runs `scripts/derive-payload.mjs`, which reads
`[i18n]` and `[brand]` from `fiducial.toml` and writes
`apps/web/src/generated/payload.ts`.

`pipelines/payload-codegen.toml` runs `scripts/derive-payload-codegen.mjs`
from `apps/web/` and writes `apps/web/src/payload/payload-types.ts` and
`apps/web/src/app/(payload)/admin/importMap.js`. It is **not applicable**
until `apps/web/node_modules` exists — `fid derive --check` skips it on a
fresh checkout and fails it on a machine where packages are installed but the
files are stale.

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
| `pages` | CMS pages with four blocks: `hero`, `faq`, `product-grid`, `custom-order-cta` |
| `media` | Uploaded files, stored in R2 via `@payloadcms/storage-r2` |

The global `SiteSettings` stores site-wide defaults: site name, tagline,
company info, main/footer nav menus, a promo banner, and SEO defaults.

## Blocks

Four blocks are registered on the `pages` collection:

| Block slug | File | Fields |
|---|---|---|
| `hero` | `src/payload/blocks/Hero.ts` | `heading`, `subheading`, `image` (upload), `ctaLabel`, `ctaHref` |
| `faq` | `src/payload/blocks/Faq.ts` | `heading`, `items[]` (`question`, `answer` richText) |
| `product-grid` | `src/payload/blocks/ProductGrid.ts` | `heading`, `productSlugs[]` (`slug`) |
| `custom-order-cta` | `src/payload/blocks/CustomOrderCta.ts` | `heading`, `body` richText, `image`, `ctaLabel`, `formAnchor` |

The `product-grid` block uses a `productSlugs` array field rather than a
relationship, because the `products` collection does not exist until the
`commerce` capability is installed. When `commerce` is added, replace the array
with a `relationship` field targeting `products`.

## Block renderers

Each block has a **server component** in `src/components/blocks/`:

| Component | Used for |
|---|---|
| `HeroBlock.tsx` | Full-width hero with optional background image |
| `FaqBlock.tsx` | Accordion-style FAQ list |
| `ProductGridBlock.tsx` | Grid of product links by slug |
| `CustomOrderCtaBlock.tsx` | CTA panel with optional image |
| `BlockRenderer.tsx` | Dispatches `Page.blocks[]` to the right component |

Every `<Image>` in a block renderer carries an explicit `sizes` attribute (D10).

## Revalidation

`src/payload/revalidate.ts` exports two Payload hooks:

| Hook | Used by | Does |
|---|---|---|
| `revalidatePage` | `Pages` `afterChange` | `revalidatePath("/<slug>")` + `revalidateTag("page:<slug>")` + `revalidateTag("pages")` + `revalidatePath("/")` |
| `revalidateSiteSettings` | `SiteSettings` `afterChange` | `revalidatePath("/", "layout")` + `revalidateTag("site-settings")` |

Never call `revalidate*` in a client component — these hooks run server-side.

## Images

Images are stored in R2 and resized through Cloudflare Images (`/cdn-cgi/image/…`).
`next.config.ts` sets `loader: "custom"` pointing at
`src/lib/cloudflare-image-loader.ts`. The loader maps Next.js `width`/`quality`
props to Cloudflare Images transform parameters.

This requires the **Cloudflare Images** product to be enabled on the zone. The
loader is a no-op in `output: "export"` (Cloudflare workers builds), where
`images: { unoptimized: true }` takes over automatically.

Every `<Image>` in a block renderer must carry an explicit `sizes` attribute;
omitting it triggers `next/image` warnings and prevents correct responsive sizing.

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

A Cron Trigger at `0 2 * * *` calls `POST /api/d1-backup`, which uses
`env.DB.dump()` (the D1 Workers binding) to export the full SQLite database and
writes it to `backup/<YYYY-MM-DD>.sql` in R2. No Cloudflare REST API token is
needed — the binding dump runs inside the Worker.

**Setup** — add to `[deploy]` in `fiducial.toml`:

```toml
[deploy]
crons   = ["0 2 * * *"]
secrets = ["BACKUP_TOKEN"]
```

Then run `fid derive` and `wrangler secret put BACKUP_TOKEN`.

Call the endpoint from the cron handler with:

```
POST https://<your-domain>/api/d1-backup
Authorization: Bearer <BACKUP_TOKEN>
```

If `BACKUP_TOKEN` is unset, the endpoint runs without auth (suitable for local
testing; set it in production).

---

### Restore procedure

**Last restored:** _(update this line after every restore)_

#### Option A — from an R2 nightly backup

```sh
wrangler r2 object get <bucket> backup/<YYYY-MM-DD>.sql --file restore.sql
wrangler d1 execute <database> --file restore.sql --remote
```

Replace `<bucket>` with the R2 bucket name from `[deploy] r2_bucket_name` and
`<database>` with the database name (`[deploy] d1_database_name` or the product
name).

#### Option B — D1 Time Travel (point-in-time recovery)

Cloudflare D1 preserves up to 30 days of history automatically. Use Time Travel
to restore to any second within that window without touching R2:

```sh
wrangler d1 time-travel restore <database> --timestamp <ISO8601>
```

Example — restore to two hours ago:

```sh
wrangler d1 time-travel restore <database> \
  --timestamp "$(date -u -v-2H '+%Y-%m-%dT%H:%M:%SZ')"
```

On Linux, replace `-v-2H` with `-d '-2 hours'`.

Inspect available bookmarks without restoring:

```sh
wrangler d1 time-travel info <database>
```

Time Travel is available on the Cloudflare Workers Paid plan.

## Worker limits

This capability requires the **Cloudflare Workers Paid plan** — Payload's
bundle exceeds the 1 MB free limit (Workers runs up to 64 MiB uncompressed
since September 2026).

## Node floor

This capability requires **Node ≥ 22.18**. The codegen pipeline uses
`module.registerHooks` (added in Node 22.13) and native TypeScript type
stripping (unflagged since Node 22.18). Both are needed to run the Payload
CLI without tsx.

## Rules

- Never put a value in `[deploy] secrets` — only the name.
- Never hand-edit `apps/web/src/generated/payload.ts` or the generated
  `apps/web/src/payload/payload-types.ts`.
- Collections and globals are product-owned templates, so edit them freely.
- After any schema change, run `fid derive payload-codegen` (or
  `pnpm --filter @{{name}}/web run generate:types && generate:importmap`)
  to regenerate the types and import map.

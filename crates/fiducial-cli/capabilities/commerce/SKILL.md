# Skill: commerce

**Capability:** `commerce` · **Platform:** Fiducial {{version}}

This skill is loaded by Claude Code automatically when the `commerce` capability
is installed. Read it once at session start; do not re-derive it.

---

## What this capability adds

The commerce model for a Payload-backed client shop: Products, Variants, Orders,
Quotes, Carts, and Discounts. Designed for a Serbian furniture-maker with no
customer accounts, no online payment, and no cart UI by default.

This capability requires `payload` to be installed first.

## Domain model

| Collection | Purpose |
|---|---|
| `products` | The catalog: title, description, images, base price |
| `variants` | Per-product options: dimensions, material, color; override price |
| `orders` | Placed by visitors; `new → confirmed → paid → delivered` |
| `quotes` | Made-to-measure inquiries; convertible to an order |
| `carts` | Guest carts; enabled only when `[commerce] cart = true` |
| `discounts` | Phone-agreed discounts recorded on the order; no UI |

Shared field groups:
- `fields/price.ts` — `amountMinor` (integer minor units) + `currency`
- `fields/lineItems.ts` — the shared line-item shape
- `fields/leadSource.ts` — lead source select + UTM text fields
- `fields/statusTimestamps.ts` — one `Date` field per status + `beforeChange` hook

## Development

```sh
pnpm dev --filter @{{name}}/web
```

The commerce collections appear in the Payload admin at `/admin`.

## The derive pipeline

`pipelines/commerce.toml` runs `scripts/derive-commerce.mjs`, which reads
`[commerce]` from `fiducial.toml` and writes
`apps/web/src/generated/commerce.ts`.

That module exports:

| Export | Is |
|---|---|
| `COMMERCE_CURRENCY` | The ISO 4217 currency code from `[commerce] currency` |
| `COMMERCE_CART_ENABLED` | `boolean` from `[commerce] cart` |
| `COMMERCE_STATUSES` | `readonly` ordered status array |
| `COMMERCE_LEAD_SOURCES` | `readonly` lead-source array |
| `CommerceStatus` | Union type of status values |
| `CommerceLeadSource` | Union type of lead-source values |

**Never hand-edit** `apps/web/src/generated/commerce.ts`. Change `[commerce]`
in `fiducial.toml` and run `fid derive`.

## Prices

Prices are stored as integer minor units (`amountMinor: number`) with a
`currency: string` alongside them. For RSD, `minorUnitExponent("RSD") === 2`, so
`{ amountMinor: 150000, currency: "RSD" }` is 1500.00 RSD.

Display with `formatMoney` from `@fiducial/i18n`:

```typescript
import { formatMoney } from "@fiducial/i18n";
formatMoney({ amountMinor: 150000, currency: "RSD" }, "sr", { hideZeroMinorUnits: true });
// → "1.500 RSD"
```

## Status machine

Declared in `[commerce] statuses`; `beforeChange` hook in
`fields/statusTimestamps.ts` sets `<status>At = new Date()` on each transition:

```
new → quoted → confirmed → paid → delivered
         └──────────────────────────────→ cancelled
```

Each timestamp is set **once** and never overwritten — if a document already has
`confirmedAt`, the hook skips it.

## Guest checkout fields

Orders, Quotes and Carts share these fields (declared in each collection):

| Field | Type |
|---|---|
| `guestName` | `text`, required |
| `guestPhone` | `text`, required |
| `guestEmail` | `email` |
| `guestCity` | `text` |
| `guestAddress` | `textarea` |

## Access control

All access functions come from `apps/web/src/payload/access.ts` (installed by
the `payload` capability):

| Collection | create | read | update | delete |
|---|---|---|---|---|
| `products` | `isOwnerOrStaff` | `isAuthenticated` | `isOwnerOrStaff` | `isOwner` |
| `variants` | `isOwnerOrStaff` | `isAuthenticated` | `isOwnerOrStaff` | `isOwner` |
| `orders` | public | `isOwnerOrStaff` | `isOwnerOrStaff` | `isOwner` |
| `quotes` | public | `isOwnerOrStaff` | `isOwnerOrStaff` | `isOwner` |
| `carts` | public | `isOwnerOrStaff` | `isOwnerOrStaff` | `isOwner` |
| `discounts` | `isOwnerOrStaff` | `isOwnerOrStaff` | `isOwnerOrStaff` | `isOwner` |

## Admin actions

**Mark order as paid** — `POST /api/orders/:id/mark-paid`

Sets `status = "paid"` and `paidAt = new Date()`. Requires `staff` or `owner`
(verified via `can()` from `@fiducial/identity`). Returns the updated document.

**Convert quote to order** — `POST /api/quotes/:id/convert`

Creates a new `orders` document from the Quote, sets the Quote status to
`confirmed`. Requires `staff` or `owner`.

## D1 constraints

Payload's D1 adapter has known issues with array field updates (issue #15219).
Line items in Orders and Quotes are stored as a JSON `textarea` to avoid this —
`lineItemsField` in `fields/lineItems.ts` is used in Carts only. Orders and
Quotes use `lineItemsJson` (a `textarea` holding the same shape as a JSON string).

## Configuration

```toml
[commerce]
currency      = "RSD"
cart          = false
statuses      = ["new", "quoted", "confirmed", "paid", "delivered", "cancelled"]
lead_sources  = ["direct", "instagram", "facebook", "referral", "other"]
```

`cart = false` means the `carts` collection is registered but the cart UI is not
built. Set to `true` when a cart UX is added.

## Superseded files

This capability supersedes two `payload`-owned files:

| File | Why |
|---|---|
| `apps/web/payload.config.ts` | Registers the commerce collections |
| `apps/web/src/payload/blocks/ProductGrid.ts` | Uses a `relationship` to `products` instead of a slug array |

## Email notifications and the jobs queue

Orders and Quotes each fire a Payload `afterChange` hook on `create` that queues
two jobs — one to notify the owner, one to send a confirmation to the customer:

| Task slug | Input | What it sends |
|---|---|---|
| `notify-new-order` | `orderId`, `ownerEmail` | Owner alert + customer confirmation |
| `notify-new-quote` | `quoteId`, `ownerEmail` | Owner alert + customer confirmation |

Both tasks use the `email` slot from `createAdapters(env)`. Select `resend` as
the email vendor in `[adapters]` and set `RESEND_API_KEY` with
`wrangler secret put`.

**Required secrets** (`wrangler secret put <NAME>`):

| Secret | Purpose |
|---|---|
| `RESEND_API_KEY` | Email sending via Resend |
| `OWNER_EMAIL` | Recipient for owner notification emails |
| `CF_CRON_SECRET` | Authorizes the cron-triggered `POST /api/jobs/run` |
| `DOMAIN` | Optional sender domain; defaults to `example.com` |

**Queue processing** — slow work is deferred; a cron drains the queue.
Add to `[deploy]` in `fiducial.toml`:

```toml
[deploy]
crons   = ["0 */6 * * *"]
secrets = ["RESEND_API_KEY", "OWNER_EMAIL", "CF_CRON_SECRET"]
```

Then configure a Cloudflare Worker cron trigger that calls:

```
POST https://<your-domain>/api/jobs/run
Authorization: Bearer <CF_CRON_SECRET>
```

Or add a `scheduled` handler in `open-next.config.ts` that calls the same
endpoint internally when the cron fires.

The endpoint processes up to 25 pending jobs per call. Increase the `limit`
in `apps/web/src/app/api/jobs/run/route.ts` for high-volume products.

## Rules

- Never hand-edit `apps/web/src/generated/commerce.ts`.
- Prices are always minor units; display with `formatMoney`.
- Do not add a `customers` collection — there are no customer accounts.
- Do not add a payment adapter — orders reach `paid` by an admin action only.
- The `discounts` collection has `admin: { hidden: true }` on purpose.

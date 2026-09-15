# Identity capability — grant storage

> Installed by `fid add identity`. Generates `migrations/0001_grants.sql` from
> the identity model, and gives `@fiducial/identity` somewhere to read and
> write permission rows.

## What a grant is

Auth answers *who you are*. A grant answers *what you may do*. It is one row:

| principal | resource | role |
|---|---|---|
| `user:alice` | `device:thermostat` | owner |
| `user:bob` | `device:thermostat` | viewer |

`can(principal, action, resource, grants)` reads those rows and returns
yes or no. This capability is where the rows live.

## The loop

```sh
fid add identity      # seeds [identity], installs the pipeline
fid derive            # writes migrations/0001_grants.sql
wrangler d1 execute <db> --file=migrations/0001_grants.sql
```

```ts
import { SqlGrantStore, can, principalFromSession } from "@fiducial/identity";

const who = principalFromSession(await auth.getSession());
const store = new SqlGrantStore(adapters.database);

// Fetch once per request, then answer as many questions as you like.
const grants = await store.grantsFor(who);
if (!can(who, "write", thermostat, grants)) return forbidden();
```

Granting and revoking:

```ts
await store.grant(bob, thermostat, "viewer");   // share
await store.revoke(bob, thermostat);            // unshare
await store.grantsOn(thermostat);               // who can see this?
```

## The schema is derived, not written

The table's `CHECK` constraints **are** the `Principal`, `Resource` and `Role`
variants. Add a role and forget the migration, and `fid derive --check` fails
rather than letting the two drift.

Two rules the rule engine enforces are enforced by the schema too, so an
unusable row cannot even be stored:

- **`anonymous` is not a valid `principal_kind`** — `can()` refuses an
  unidentified principal, so a grant to anonymous could never be honoured.
- **`principal_id GLOB '*[^0]*'`** — an all-zero id is the uninitialized
  sentinel at every level. An unprovisioned device must not hold a grant as
  "device zero," an identity every unprovisioned device shares.

## This is not a migrations system

It generates the **first** table. The moment your schema changes you need
ordering, idempotency and drift detection against a live database, and none
of that exists yet. Said plainly here rather than discovered later — see
`docs/specs/2026-09-15-grant-storage.md`.

## Do you need this?

If permission is always "you own what you created,"
`SELECT … WHERE owner_id = ?` answers authorization and you do not need
grants. They earn their place when **sharing** exists — Bob can view Alice's
thermostat, an installer holds temporary admin — which is where scattered
`if` statements stop working.

## Storage is a server concern

There is no Rust counterpart: `fiducial-identity` is `no_std` and a device
cannot run SQL. A device *receives* the grants it needs rather than querying
for them. `can()` stays the shared half.

## Tests

```sh
pnpm --filter @fiducial/identity test   # incl. the real schema on real SQLite
cargo test -p fiducial-cli identity_    # end-to-end through the real binary
```

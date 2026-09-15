# Identity capability — grant storage

> Installed by `fid add identity`. Generates `migrations/0001_grants.sql` from
> the identity model — for SQLite or Postgres, whichever your `database`
> vendor implies — and gives `@fiducial/identity` somewhere to read and write
> permission rows.

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
fid derive            # writes the migration and src/identity.generated.ts
wrangler d1 execute <db> --file=migrations/0001_grants.sql   # or: psql -f
```

```ts
import { can, principalFromSession } from "@fiducial/identity";
import { grantStore } from "./identity.generated.js";

const who = principalFromSession(await auth.getSession());
const store = grantStore(adapters.database);

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
- **at least one non-zero character in `principal_id`** — an all-zero id is
  the uninitialized sentinel at every level. An unprovisioned device must not
  hold a grant as "device zero," an identity every unprovisioned device
  shares. (`GLOB '*[^0]*'` on SQLite, `~ '[^0]'` on Postgres.)

## Two dialects, because "SQL" is not one language

The dialect is **derived, not declared**: `[adapters] database` already names
the vendor, and the vendor implies it — `d1` is SQLite, `supabase` / `neon` /
`postgres` are Postgres. `[identity] dialect` overrides that for a product the
platform cannot see (`database = "none"` pointing at a database of its own).

The two differ where this touches them, and each rejects the other's spelling:

| | SQLite | Postgres |
|---|---|---|
| "has a non-zero character" | `GLOB '*[^0]*'` | `~ '[^0]'` |
| timestamp | `TEXT` | `TIMESTAMPTZ` |
| bound parameter | `?` | `$1` |

The last one is why `fid derive` also writes **`src/identity.generated.ts`**:
it carries the resolved dialect and table name into TypeScript, so
`grantStore(db)` builds a store that speaks to the migration beside it.
Constructing `new SqlGrantStore(db)` by hand against a Postgres table gets you
a syntax error on every query, at runtime.

## Row-level security (Postgres)

```toml
[identity]
rls = true
current_user_sql = "auth.uid()"   # the default; Supabase's
```

Adds policies: a principal reads its own grants, an administrator of a
resource reads every grant over it, and only an admin or owner may write one.

This is **defence in depth, not the rule**. `can()` stays the rule — it is the
one both languages share, the one the conformance vectors check, and the only
one firmware can run. What RLS adds is that a missed check in a handler stops
being a data breach.

Two things worth knowing before you turn it on:

- The lookup is a `SECURITY DEFINER` function, not an inlined `EXISTS`. A
  policy on `grants` that reads `grants` re-enters itself, and Postgres
  refuses the query with *"infinite recursion detected in policy"* — at query
  time, not at `CREATE POLICY` time.
- **The table's owner is not subject to its policies**, and this deliberately
  does not emit `FORCE ROW LEVEL SECURITY` (that would put the helper function
  back inside the recursion). A server-side connection as the owner — or as
  Supabase's `service_role` — administers grants freely. That is the intended
  split, and it means RLS protects you from the *client's* connection, not
  from your own handler.

`rls = true` on SQLite is refused at derive time rather than ignored: SQLite
has no row-level security.

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
pnpm --filter @fiducial/identity test          # the rule, on the vectors
pnpm --filter @fiducial/identity test:schema   # the real schema on real SQLite
cargo test -p fiducial-cli identity_           # end-to-end through the real binary
PGHOST=… scripts/verify-postgres.sh            # migration + policies on real Postgres
```

`test:schema` and the Postgres script need `cargo build -p fiducial-cli --bin
fid` first — they run what `fid derive` actually produced, not a copy.

Every one of those runs the schema `fid derive` actually generates, rather
than a copy. The Postgres script is what caught both a migration that would
not apply and policies that refused every query they guarded.

# Grant storage: the schema is derived from the identity model

**Date:** 2026-09-15
**Status:** accepted

---

## Context

`docs/specs/2026-09-15-identity-at-every-level.md` shipped `Principal`,
`Grant` and `can(principal, action, resource, grants)`, and deliberately did
not ship anywhere for the grants to live: *"where grants live — a D1 table,
Postgres RLS policies, a device's flash — is a product's decision, and
inventing a persistence layer here with no consumer is the
speculative-contract mistake this platform has already corrected once."*

That was the right call at the time and it left identity **unusable**. `can()`
takes a list nothing produces. A product would have had to hand-assemble the
rows on every request, which is the scattered-`if` problem the rule existed to
delete, moved one level down.

It also turned out to be the least-understood part of the platform. Asked
three times what a grant even was, the answer that landed was the smallest
one: *a grant is one row — who, over what, how much.*

| principal | resource | role |
|---|---|---|
| `user:alice` | `device:thermostat` | owner |
| `user:bob` | `device:thermostat` | viewer |

## Decision

### Not a ninth contract — a consumer of the `database` one

Grants are rows in the database the product already selected. So
`SqlGrantStore` calls `query`/`execute` and nothing else, which means it works
on D1 today and on any future `database` vendor for free. Adding a ninth
adapter contract for "permissions" would have been a second vendor axis for
something that is not vendor-shaped.

It takes the database **structurally** — a `{ query, execute }` shape rather
than an imported `Database` — so `@fiducial/identity` keeps depending on
nothing, and a real `Database` satisfies it by having the right methods. Same
move as `principalFromSession` taking an id rather than an `AuthSession`.

### The schema is derived from the identity model

`fid derive` (the `fid-identity` executor) generates
`migrations/0001_grants.sql`. Its `CHECK` lists **are** the `Principal`,
`Resource` and `Role` variants:

```sql
principal_kind TEXT NOT NULL CHECK (principal_kind IN ('user', 'device', 'service')),
role           TEXT NOT NULL CHECK (role IN ('viewer', 'member', 'admin', 'owner')),
```

Add a role and forget the migration, and `fid derive --check` fails. Ship the
schema as a hand-written `.sql` template instead, and the first new role is a
table that silently rejects it. Same reasoning as `wrangler.toml` one phase
earlier: the fact was already declared, and copying it is where it goes wrong.

### Two runtime rules, enforced by the schema as well

`can()` refuses an unidentified principal. The table refuses to *store* one:

- **`anonymous` is absent from `principal_kind`'s CHECK.** A grant to
  anonymous could never be honoured, so it cannot be written.
- **`principal_id GLOB '*[^0]*'`** — at least one non-zero character. An
  all-zero id is the uninitialized sentinel at every level of this platform;
  without this, an unprovisioned device could hold a grant as "device zero,"
  an identity every unprovisioned device shares.

Enforcing the same rule twice is usually the duplication this platform
deletes. Here it is deliberate and the exception is worth naming: these are
the only two places the decision can be made — at the gate, and at the row —
and a row that the gate would ignore is garbage that should never have been
written. Both derive from the same model, so they cannot disagree.

### One role per (principal, resource)

`PRIMARY KEY (principal_kind, principal_id, resource_kind, resource_id)` with
`ON CONFLICT … DO UPDATE`. Granting again *replaces*. A resource-specific
grant and a `platform`-scoped grant are different rows, so a principal can
hold both and `effectiveRole` takes the stronger — which is why `effective_role`
returns the max rather than the first match.

### Tested against real SQLite, on the real generated schema

`packages/identity/src/grants.test.js` runs `node:sqlite` — **D1 is SQLite**,
so this is the same engine and the same DDL a deployed product runs. The
schema is not a copy checked into the test: the suite shells out to the real
`fid` binary, runs `fid add identity && fid derive`, and reads what came out.
A checked-in copy would be exactly the second declaration this pipeline
exists to delete.

Both stores — SQL and in-memory — run the *same* suite, because a difference
between them is a bug in whichever one a product is not using in production.

**Verified adversarially.** Changing one column name in the store (`principal_id`
→ `principal_uid`) fails 10 of 35 tests. Against a mocked database every query
"works": a typo'd column, a broken `ON CONFLICT`, a `CHECK` that rejects
nothing all pass. That is the failure mode this test design exists to avoid.

## Why not the alternatives

**A `permissions` adapter contract with vendor implementations.** Rejected:
permissions are not vendor-shaped. There is no "Supabase permissions" versus
"D1 permissions" — there are rows, in whatever database you picked. A contract
here would be a second axis of choice over something already chosen.

**Postgres RLS / D1 policies instead of an application-level check.** RLS is
genuinely good and genuinely vendor-specific — it would put the authorization
rule inside one vendor's engine, where the `no_std` firmware half cannot reach
it and where the conformance vectors cannot check it. The whole point of
`fiducial-identity` is one rule at every level. A product may of course add RLS
underneath as defence in depth.

**Grants as JWT claims, avoiding the query entirely.** Faster — no round trip —
and the reason it is not the default is revocation: a claim is true until the
token refreshes, so "remove Bob's access" would take effect minutes later.
Trading a latency win for a delayed revocation is a real decision a product can
make deliberately; it should not be the shape the platform forces.

**Shipping `0001_grants.sql` as a capability template rather than deriving it.**
This is the near-miss. A template is hand-editable and three-way-merged on
upgrade, which sounds friendlier — and means the first added `Role` produces a
table that rejects it, in production, with a `CHECK constraint failed` that
names nothing useful.

## Consequences

- Identity is usable end to end: `getSession` → `principalFromSession` →
  `grantsFor` → `can`. Every step now exists.
- `[identity] table` is a new declaration — the one fact about grant storage
  that the model cannot imply. It reaches SQL by interpolation (a parameter
  cannot bind an identifier), so it is validated as an identifier in **both**
  the executor and `SqlGrantStore`, and a name like `grants; DROP TABLE users`
  is refused at derive time.
- 35 TypeScript tests (SQL and in-memory against one suite), 8 end-to-end CLI
  tests. The `identity` CI job now builds `fid` first, because the schema
  tests use the real one.

## What this deliberately does not do

- **It is not a migrations system, and the pipeline's own comment says so in
  capitals.** It generates the *first* table. The moment the schema changes
  you need ordering, idempotency, and drift detection against a live
  database — none of which exists. That is the next real design question here,
  and it is bigger than this was.
- **No Rust counterpart.** `fiducial-identity` is `no_std`; a device cannot run
  SQL. A device *receives* the grants it needs rather than querying for them —
  and how they get pushed over the protocol into flash is its own pass.
- **No grant expiry, no delegation, no audit log.** Each is additive to the
  row shape; none has a consumer yet.
- **No caching.** `grantsFor` is documented as fetch-once-per-request. A cache
  is the right optimization at the point someone measures it, with revocation
  semantics decided knowingly rather than inherited.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

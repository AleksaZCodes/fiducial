# Two SQL dialects, and RLS as defence in depth

**Date:** 2026-09-15
**Status:** accepted
**Supersedes in part:** `docs/specs/2026-09-15-grant-storage.md` — the
"Postgres RLS / D1 policies instead of an application-level check" rejection.

---

## Context

`docs/specs/2026-09-15-grant-storage.md` shipped the grants table one phase
earlier and called it a derivation of the identity model. It was — of a model,
into one dialect, silently:

> **`principal_id GLOB '*[^0]*'`** — at least one non-zero character.

`GLOB` is SQLite. On Postgres that is not a subtly different behaviour, it is
a parse failure:

```
ERROR:  syntax error at or near "GLOB"
```

So a product on Supabase or Neon could run `fid derive` and get a migration
that does not apply. The generated file said nothing about which engine it was
for, so there was nothing to notice.

The prompt was one sentence: *"but postgresql works different from sqlite, and
postgres has row level policies. I usually prefer postgres."* Both halves were
right, and the first was a bug.

The same spec also declined RLS:

> RLS is genuinely good and genuinely vendor-specific — it would put the
> authorization rule inside one vendor's engine, where the `no_std` firmware
> half cannot reach it and where the conformance vectors cannot check it.

That reasoning holds for RLS *as the rule*. It was used to reject RLS *at all*,
which does not follow — and the same paragraph had already conceded the point
in its last line ("A product may of course add RLS underneath as defence in
depth"), while leaving the product to hand-write it.

## Decision

### The dialect is derived from the vendor, not declared

`[adapters] database` already names the vendor and the vendor implies the
dialect: `d1` → SQLite, `supabase` / `neon` / `postgres` → Postgres. Asking for
the dialect separately would be the second declaration this platform exists to
delete, and the one that goes stale when a product switches vendors.

`[identity] dialect` exists as an **override** for the case the platform cannot
see: `database = "none"` in front of a database the product connects to itself.

The differences that matter are few and total — each engine rejects the other's
spelling outright:

| | SQLite | Postgres |
|---|---|---|
| "has a non-zero character" | `GLOB '*[^0]*'` | `~ '[^0]'` |
| timestamp | `TEXT` | `TIMESTAMPTZ` |
| bound parameter | `?` | `$1` |

The generated file now names its dialect in a header comment, so an applied
migration and the engine it was applied to can be compared by eye.

### The third row is why `fid derive` now emits TypeScript too

Placeholders are not in the migration; they are in `SqlGrantStore`, which
hard-coded `?`. A product on Postgres would have got a migration that applies
and then a syntax error on every single query — at runtime, in production.

Fixing the store to take a dialect and leaving the product to pass it is the
same trap one level along: a fact declared in `fiducial.toml` and retyped in
TypeScript. So the pipeline gained a second output,
`src/identity.generated.ts`, carrying the two facts a store needs:

```ts
export const grantsTable = "grants";
export const sqlDialect: SqlDialect = "postgres";
export function grantStore(db: GrantDatabase): SqlGrantStore { … }
```

The table name had the same latent problem — `new SqlGrantStore(db, "grants")`
was a hand-copy of `[identity] table` — and this fixes both.

The store keeps **one** copy of each query, written in SQLite's spelling and
renumbered for Postgres (`?` → `$1`, `$2`). Two hand-written variants of one
query is two declarations, and they drift. The rewrite is textual, which is
safe here specifically — every query is a literal in that file and none
contains a `?` outside a placeholder — and is documented as not being a
general-purpose translator.

### RLS: defence in depth, generated from the same model

`rls = true` (Postgres only) generates policies:

- a principal may read its own grants;
- a principal may read every grant over a resource it administers;
- only an admin or owner of a resource may write grants over it.

This does not make RLS the rule. `can()` stays the rule: it is the one both
languages share, the one `docs/identity/vectors.json` checks, and the only one
firmware can run — a `no_std` device reaches no Postgres engine. What the
policies add is that a missed check in a handler stops being a data breach.

Enforcing the same decision twice is normally the duplication this platform
deletes. It is safe here for the reason the schema's `CHECK` constraints are
safe: **both are generated from the same model**, so they cannot disagree.

`rls = true` on a SQLite dialect is refused at derive time rather than quietly
ignored. A security control that silently does nothing is worse than one you
know you do not have.

### The `EXISTS` that cannot be an `EXISTS`

The obvious spelling of "does this user administer this resource?" inside the
policy is a subquery over `grants`. It is also wrong, and wrong in a way that
`CREATE POLICY` accepts without complaint:

```
ERROR:  infinite recursion detected in policy for relation "grants"
```

A policy on `grants` that reads `grants` re-enters itself. This is what the
first draft of the generator emitted; it parsed, applied cleanly, and then
refused every query it guarded.

The lookup lives in a `SECURITY DEFINER` function instead, which runs as the
table's owner — who is not subject to the table's policies — so the inner read
completes and the recursion never starts. Two consequences are deliberate:

- **No `FORCE ROW LEVEL SECURITY`.** Forcing policies onto the owner would put
  the helper back inside the loop it exists to break. So the owner (and
  Supabase's `service_role`) administers grants freely: RLS here protects you
  from the client's connection, not from your own trusted handler. That is the
  Supabase deployment model, stated rather than assumed.
- **`SET search_path = ''` with fully-qualified names.** A `SECURITY DEFINER`
  function that inherits the caller's search path lets any caller shadow the
  objects it names and run their own code as the owner. This is a known
  privilege-escalation shape, not a style preference.

### Verified against a real PostgreSQL server

`packages/identity/src/grants.test.js` runs the generated schema on real SQLite
because D1 *is* SQLite. Node has no Postgres, so the other half had no engine
behind it — which is exactly how a migration that does not apply, and policies
that refuse every query, both got written and read as correct.

`scripts/verify-postgres.sh` runs it: `fid derive` → apply the migration →
exercise the policies from each side, and the store's own SQL in the shape
Postgres numbers it. Fourteen checks, against PostgreSQL 16. The `identity` CI
job starts a `postgres:16` service and runs it on every push.

It reproduces both defects when the fixes are reverted: the `GLOB` migration
fails to apply, and the inlined-`EXISTS` policy turns every read into the
recursion error.

## Why not the alternatives

**Generate ANSI SQL that runs on both.** There is no portable spelling of the
zero-sentinel check, and none of `ON CONFLICT` with the semantics wanted here.
Generating the subset both accept means generating a weaker table on both.

**Ask for the dialect in `fiducial.toml`.** That is the vendor declared twice.
A product that switches `database` would silently keep generating the old
dialect — the failure this platform exists to make impossible.

**Two hand-written query sets in `SqlGrantStore`.** Same objection, inside one
file: the Postgres copy is the one nobody runs locally, so it is the one that
goes stale.

**A `pg` dependency so the TypeScript suite can test Postgres directly.**
`@fiducial/identity` has zero runtime dependencies and that is worth keeping. A
shell script against a real server tests more of the truth anyway — it exercises
the migration, the policies and the role separation, none of which a store
mock would touch.

**RLS as the authorization rule, replacing `can()`.** Still rejected, for the
original spec's reason: firmware cannot reach a Postgres engine, and a rule the
conformance vectors cannot check is a rule that can drift from the Rust half.

## Consequences

- A Postgres product gets a migration that applies and a store that queries it.
  Before this, it got neither.
- `fid derive` writes two files for the identity pipeline. Outputs are matched
  by extension rather than position, so declaring them in either order works.
- Three new declarations, all optional and all Postgres-shaped: `dialect`,
  `rls`, `current_user_sql`.
- CI gained a `postgres:16` service on the `identity` job.
- 18 end-to-end CLI tests (was 8), 206 TypeScript tests (was 202), 14 live
  Postgres checks (was 0).

## What this deliberately does not do

- **It is still not a migrations system.** Turning `rls` on after the table is
  live regenerates `0001_grants.sql`, which is the *first* migration; applying
  it to a live database is the product's problem, and the same open question
  the previous spec named.
- **No Postgres `database` adapter.** `supabase` / `neon` / `postgres` are
  still `candidates` in the `database` contract — declaring one resolves the
  dialect correctly, but the runtime adapter does not exist yet, so the product
  supplies its own `{ query, execute }`.
- **The policies cover the grants table only.** A product's own tables are its
  own to protect; `grants_administers(kind, id)` is exported as a plain
  function precisely so those policies can call it.
- **No `service` or `device` principal in the policies.** They read the current
  *user*, because `auth.uid()` is a user's id. A device or service connecting
  directly to Postgres is a different authentication story and has no consumer.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

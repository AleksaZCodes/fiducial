# Schema migrations: ordering, idempotency, drift

**Date:** 2026-09-15
**Status:** implemented
**Closes:** the gap `docs/specs/2026-09-15-grant-storage.md` named and declined

---

## Context

Grant storage generated the first table and said so plainly:

> **This is not a migrations system.** It generates the first table. The moment
> the schema changes you need ordering, idempotency and drift detection against
> a live database, and none of that exists yet.

That was the right call then and the wrong state to stay in: a product with
`migrations/0001_grants.sql` and no way to apply a second one has a schema it
can create once and never change.

## Decision

`migrations/NNNN_slug.sql` is the declaration. `fid derive` generates
`src/migrations.generated.ts` — the migrations in order, each with its SQL
embedded and hashed. `Migrator` in `@fiducial/adapters/migrate` applies them
through the `database` contract.

**Not the same thing as `migration.rs`,** which holds codemod migrations `fid
upgrade` applies to source files. They share a word and nothing else, so the
distinction is in the module names (`schema.rs` vs `migration.rs`), the
executor name, and every user-facing string.

### The three properties, and why none is optional

- **Ordering.** `0002` may reference what `0001` created. Sorting is numeric,
  not lexical: `9` before `10` is a set that applies in the wrong order for a
  product that drops the zero padding.
- **Idempotency.** Deploys re-run. A ledger table records what was applied, and
  the migration and its ledger row go in one batch — a migration that ran
  without being recorded re-runs on the next deploy.
- **Drift.** A migration edited *after* being applied is the dangerous one:
  environments that ran it keep the old schema, new ones get the new one, and
  neither can tell they disagree. This is why the ledger stores a hash, not
  just an id.

### Why the SQL is embedded rather than read

The runner has no filesystem. A Worker applying migrations at deploy time
cannot open `migrations/`. Embedding is also what makes the hash mean anything:
it is taken over the text that actually shipped, rather than over a file the
runtime never saw.

### Drift is reported, never repaired

There is no safe automatic answer. The database already has whatever the old
text did, and re-running the new text may or may not be valid against it. A
person has to decide; the only useful thing the runner can do is make sure they
know. `apply()` refuses to run *at all* while any applied migration has
drifted, because adding migrations on top of a schema nobody has reconciled
compounds a problem that has not been looked at.

`status()` returns drift rather than throwing: it is what you run to find out
you have a problem, so it cannot be the thing that refuses to tell you.

## Why not the alternatives

**Use a vendor's migration tool** (wrangler d1 migrations, supabase db push).
Each is vendor-specific, which is the lock-in the adapter contract exists to
prevent — and a product that switches database vendors would have to rewrite
its migration history rather than its adapter selection.

**Timestamps instead of sequence numbers.** They avoid renumbering collisions
between branches, at the cost of an order nobody can read. The collision is
caught at generation time here, loudly, which is the cheaper trade for a
one-person-plus-agents workflow.

**Let the runner repair drift by re-applying.** It cannot know whether the new
text is valid against what the old text produced. Re-applying a `CREATE TABLE`
fails; re-applying an `ALTER` may succeed and produce a schema matching neither.

## Consequences

A defect found while building, and the reason a fourth mechanism exists:
`fid derive --check` hashes declared *outputs* against the lock, so adding
`migrations/0003_x.sql` and forgetting to re-run `fid derive` left a manifest
byte-identical to what the lock recorded. The check passed and the migration
silently never ran — precisely the class of failure this system exists to
prevent. `outputs_with_moved_inputs` regenerates the manifest and compares.
Only `fid-schema` is covered: it is a pure function of `migrations/`, where the
other executors either read declarations the lock already tracks or shell out
to tools that cannot be re-run for free.

The conformance rule *"a capability that installs pipelines must declare what
they read"* had no way to express this capability's input: it ships no
migration, and cannot, because which migrations a product needs is the
product's business. `Declaration::Directory` names the directory without
inventing a placeholder — a dummy `0000_init.sql` would be a migration that
runs. It is path-validated like any other declaration, since it creates a path
in the product.

## What this deliberately does not do

- **No down migrations.** A reversal that has never been run in anger is a
  reversal that does not work, and the honest answer for a forward-only system
  is a new migration.
- **No `fid migrate apply` from the CLI.** Reaching a product's live database
  is vendor-specific and needs credentials; the runner goes through the
  `database` contract the product already configured, which is where those
  credentials already are.
- **No cross-environment reporting.** Knowing that staging is two migrations
  behind production needs both databases, which is a deploy concern.
- **No transactional DDL guarantee.** SQLite and Postgres differ, and the
  runner uses the vendor's `batch`. What it promises is what the contract
  promises.

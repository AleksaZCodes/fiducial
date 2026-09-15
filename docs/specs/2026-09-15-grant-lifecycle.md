# Grants have a lifecycle: expiry, delegation, audit, caching

**Date:** 2026-09-15
**Status:** implemented

---

## Context

`docs/specs/2026-09-15-identity-at-every-level.md` shipped a grant as three
fields — principal, resource, role — and listed what it left out: "no grant
expiry, delegation, audit log or caching."

Each absence has a failure mode, and they are not hypothetical:

- **No expiry.** A support engineer is granted `Admin` to debug something. The
  grant is permanent, because revoking it depends on someone remembering. A
  grant that has to be remembered to be revoked is one that is not revoked.
- **No delegation.** Only the platform can issue grants, so every "let my
  colleague in" is an operator ticket. Flattening it — issuing the grant
  directly — loses the link, and revoking the manager leaves everything they
  asked for in place.
- **No audit.** The verdict is computed and thrown away. Afterwards, nobody can
  say why someone was refused, because the grants table has moved on.
- **No caching.** `can()` is O(grants) and a Worker evaluates it per request
  against a table fetched per request.

## Decision

### Expiry fails closed without a clock

`Grant.expires_at` is `Option<Timestamp>`, and `can()` takes
`now: Option<Timestamp>`. `None` means *this caller has no clock*.

A permanent grant is unaffected. A grant with an expiry is **refused**. This is
the important half: firmware is a first-class target here, and a device whose
RTC has not synced is exactly a caller with no clock. Treating "no clock" as
"not expired" would make expiry a suggestion that evaporates in the one
environment least able to notice it had.

### A delegated grant is evaluated, not flattened

`Grant.delegated_by` records who issued it. At evaluation time, the delegator
must themselves be able to `Admin` the resource — the action this platform
defines as "change who else may act", so a `Member` cannot mint grants they
could not use.

Storing the link rather than flattening it is the whole point: revoke the
manager and every grant they issued stops working, without anyone having to go
and find them. It also composes with expiry — the chain is live only while
every link in it is.

The walk is depth-bounded at `MAX_DELEGATION_DEPTH = 4`. A cycle (A delegated
by B, B delegated by A) is a grant table anyone with `Admin` can write, and an
unbounded walk there is a stack overflow in the authorization path: a crash on
firmware, a denial of service everywhere else.

### The audit record is the reason, produced at decision time

`explain()` returns `Decision { allowed, reason, via }`. `can()` is now
`explain(...).allowed`, so there is one rule and not two.

The reason only exists at the moment of decision. Reconstructing it afterwards
from the grants table is guesswork, because the table has moved on by the time
anyone reads the log. Where several grants fail, the **most specific** denial
is reported: "expired" sends a reader to renew, where "no grant" would send
them to create one that is already there.

The crate has no sink, writer or buffer. `no_std` has nowhere to put one, and a
Worker, a desktop app and a device each have a different right answer for where
a record goes. The crate produces the record; `AuditLog` in
`@fiducial/identity` stores it, against a table generated as
`migrations/0002_audit.sql` when `[identity] audit = true`.

**That table is a second migration, never an edit to the first.** Editing an
applied migration is the one thing `Migrator.apply()` refuses outright, and a
generator that exempted itself from the rule it generates for would be
producing exactly the drift it warns about.

### The cache holds no clock

`DecisionCache` caches the verdict, not the grants. It expires nothing by time:
a cache that decided when its own contents were stale would be a second,
quieter copy of the expiry rule, and the two would disagree — with the cache
being the one that kept saying yes. Instead `now` is part of the key, so a
different instant is a different question, and `invalidate()` is called when
the grant table changes.

## Consequences

**`can()` and `effective_role()` changed arity.** Every caller passes `now`.
This is a breaking change made deliberately rather than adding `can_at()`
beside it: two entry points where one forgets about expiry is how the rule
develops a hole, and the compiler finding every caller is the cheapest audit
available.

**A defect Postgres found and SQLite could not.** `expires_at` was generated as
`INTEGER`. Postgres's `INTEGER` is four bytes; milliseconds since the epoch is
~1.79e12, so every insert failed with *"integer out of range"*. SQLite's
`INTEGER` holds eight bytes, so the same DDL worked there and every test passed.
It is the same shape as the `GLOB` defect this platform already shipped once:
SQL is not one language, and a schema asserted as text is not a schema anyone
has tried. `scripts/verify-postgres.sh` now runs 23 checks, nine of them new.

**The vectors carry the reason, not just the verdict.** Two implementations
agreeing on "denied" while disagreeing on why have diverged in a way an audit
log records wrongly and no verdict test would catch. Verified adversarially:
inverting the no-clock rule in TypeScript alone fails three vectors.

## What this deliberately does not do

- **No renewal or grace period.** A grant is live or it is not. A grace period
  is a policy that belongs where the product's other policies are.
- **No revocation list.** Deleting the row is revocation. A tombstone table
  answers a different question — "what was revoked, when" — which is the audit
  log's job.
- **No immutability trigger on the audit table.** SQLite and Postgres disagree
  about how to forbid an `UPDATE`, and a log whose immutability is enforced
  differently on each vendor is one whose guarantee nobody can state. The class
  offers no update or delete; enforcing it against a determined writer is the
  database role's job.
- **No automatic cache invalidation.** The cache cannot see the grant table
  change. Making it able to would mean routing every write through it, which is
  a larger coupling than the problem justifies.

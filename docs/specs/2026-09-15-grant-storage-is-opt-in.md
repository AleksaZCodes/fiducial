# Grant storage is opt-in, and a switched-off pipeline is not checked

**Date:** 2026-09-15
**Status:** accepted

---

## Context

Everything in this platform is already opt-in. `fid new` scaffolds one
capability; the other eleven need `fid add`. Every adapter contract defaults to
`none`, and `none` is a real wired implementation rather than a placeholder —
"wired in, reported, does nothing." `[spine] enabled = false`.

Grant storage followed that: nothing exists until `fid add identity`. But the
question that prompted this went one level further — *is that boilerplate for
no reason?* — and one level further is where it was true. `fid add identity`
was **one opt-in doing two jobs**: installing the rule also generated storage
for it.

`can()`, `Principal` and `effectiveRole` are useful on their own. A product
whose grants are seeded from config, held in a `MemoryGrantStore`, or read from
a table it manages itself wants all of that and none of a migration. It got
`migrations/0001_grants.sql` and `src/identity.generated.ts` anyway, and had to
ignore both.

## Decision

### `[identity] storage = "none"`

Reusing the word the adapter contracts already have, for the same meaning.
`sql` is the default, because a product that installed the capability and said
nothing gets what it got before this existed.

### The *pipeline* is switched off, not the executor

The obvious implementation is to have the `fid-identity` executor write nothing
and return success. That is wrong, and `fid derive --check` is why.

`--check` re-hashes every output of every pipeline against `fiducial.lock`, and
a declared output that is missing from disk is a failure — this is the gate the
whole platform rests on. An executor that "succeeded" while producing nothing
would have to be tolerated by that gate, and the moment it is, *"produces
nothing"* becomes an excuse available to an executor that genuinely failed to
write a file.

So the switch is at pipeline selection instead, where `fid derive` and `fid
derive --check` share one list. A pipeline a declaration turns off is neither
run nor checked, and the gate for every pipeline that *is* on keeps exactly the
strength it had. It is one `partition` rather than a new concept in the artifact
engine.

`pipeline_switched_off` is a short explicit match on the executor name, not a
general mechanism. One pipeline has a reason to be optional; a framework for
the other six would be scaffolding for consumers that do not exist.

### Declarations are validated before anything is switched off

The first version put validation where it already lived — in the executor. With
the pipeline off, the executor never runs, so `rls = true` alongside `storage =
"none"` passed in silence: a security control that cannot do anything, accepted
without a word.

Validation now runs for any product whose pipelines include `fid-identity`,
before the partition. Turning off a pipeline must not also turn off the
checking of the settings that configure it.

### The lock forgets; the disk keeps

Switching storage off drops those artifacts from `fiducial.lock`, which would
otherwise claim to keep fresh something nothing derives.

The files stay. A migration a product has already applied to a live database is
not this tool's to delete, and there is no way for it to know whether that
happened.

## Why not the alternatives

**A second capability — `fid add identity` for the rule, `fid add
identity-storage` for the table.** This was the alternative considered, and it
is more surface for the same result: two names, two skills, an upgrade path for
every product that already installed `identity`, and a second thing to
discover. The declaration is one line in a file the product already has.

**Leave it alone — installing a capability you only half want is cheap.** It is
cheap in bytes and not in trust. A generated migration a product never applies
is a file that looks like part of the system and is not, and the next person to
read the repo cannot tell which.

**Have the executor write nothing.** Rejected above: it buys the same behaviour
by weakening `--check` for everything.

**Delete the generated files when storage is switched off.** Rejected: the tool
cannot know whether that migration has been applied, and deleting the record of
a schema someone is running is not a reversible mistake.

## Consequences

- `fid add identity` installs the rule; `storage` decides whether it also
  generates storage. Default unchanged, so no product's behaviour moves.
- `table`, `dialect` and `current_user_sql` are inert under `none`, and
  documented as such. `rls = true` is refused with the reason.
- 25 end-to-end CLI tests for this pipeline, up from 18.

## What this deliberately does not do

- **No general "optional pipeline" mechanism.** One executor is named in one
  match arm. The second consumer is when to generalize, not the first.
- **It does not remove the capability's skill or package.** `@fiducial/identity`
  still ships `SqlGrantStore` — a product on `none` may well construct one by
  hand against its own table. What it stops doing is *generating* the table.
- **No migration for products already on the old behaviour**, because there is
  nothing to migrate: the default is what they already had.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

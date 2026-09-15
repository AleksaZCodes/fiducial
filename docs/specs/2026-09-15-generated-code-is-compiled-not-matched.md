# Generated code is compiled, not string-matched

**Date:** 2026-09-15
**Status:** implemented
**Supersedes:** nothing — closes a gap `2026-09-15-auth-contract.md` recorded

---

## Context

The Auth spec closed with a gap it named rather than fixed:

> **No automated typecheck of the generated factory in CI.** The bug above was
> caught by hand this round, not by a new test. […] Recorded here so it is a
> known gap, not a silently accepted one.

The bug it refers to: `NoneEmail` and `NoneDiagnostics` had no explicit
constructor, so the `new NoneEmail(env)` that `fid derive` emits into every
product's `src/adapters.generated.ts` did not compile under `strict`. It
shipped in Phase 27 and survived until Auth — through a full phase, with CI
green the whole time.

CI was green because every test on that file reads it as **text**.
`crates/fiducial-cli/tests/adapters_pipeline.rs` asserts
`factory.contains("new NoneEmail(env)")` eleven times. That assertion is true
of code that does not compile, and it was.

This is the same shape the identity work hit from the other side, and it was
already written down there: a grants migration asserted as text turned out to
use `GLOB`, which is SQLite syntax and a syntax error on Postgres, so a
Supabase product could not apply its own derived schema. Three defects, none
visible to a test that reads generated text.

Two independent occurrences of one failure mode is a pattern, not bad luck.

## Decision

`packages/adapters/src/generated-factory.test.js` runs the real generator and
hands the result to the real compiler. For each of two vendor selections — every
contract `none`, and every contract set to a vendor that is really implemented —
it scaffolds a product with `fid`, writes the `[adapters]` block, runs
`fid derive`, and runs `tsc --noEmit` over the output.

The package is resolved through `node_modules` and its own `exports` map, not a
tsconfig `paths` alias. The generator emits subpath imports
(`@fiducial/adapters/database`, `/auth`), which are exports-map entries; an
alias would typecheck a resolution no consumer actually performs.

A new CI job, `adapters`, runs it, because the suite needs both a built Rust
binary and a built JS package.

**Verified adversarially.** With the `NoneEmail` constructor deleted — the
original defect, restored — the suite fails with the compiler's own words:
`error TS2554: Expected 0 arguments, but got 1`. A gate that has never failed
is a gate nobody has tested.

## Why not the alternatives

**Add it to `pnpm test`.** It would fail there. The generic `JS/TS` job builds
no Rust, so `fid` is not on disk, and the suite would die on an ENOENT that says
nothing about what to do. That exact failure already happened once, to the
identity job's first run, and `test:schema` exists as a separate script because
of it. A suite that cannot run where it is invoked is worse than one that is
merely named.

**Run `tsc` from the Rust integration test.** The Auth spec's own sketch, and it
inverts the dependency: the Rust suite would need the JS package built first,
making `cargo test` depend on `pnpm build` for one assertion. Driving it from
the JS side keeps the edge pointing the way it already points everywhere else.

**Typecheck a committed sample factory.** Cheaper, and it re-creates the bug at
one remove: the sample is a copy of generated output, so it is correct on the
day it is committed and drifts from the generator from then on.

## Consequences

Promoting a vendor from `candidates` to `implementations` in `adapter.rs` now
has a place that must be updated — `REAL_VENDORS` in the suite — or the new
vendor's import is generated and never compiled. The suite says so in a comment,
because that is the whole point of it rather than an inconvenience.

CI gains a job that builds both toolchains. It is the second such job; the
`identity` job already does, for the same underlying reason.

## What this deliberately does not do

- **The eleven `contains()` assertions in `adapters_pipeline.rs` stay.** They
  check *which* vendor was selected, which is a different question from whether
  the result compiles, and they run without a JS toolchain. The claim here is
  that text assertions are not sufficient, not that they are worthless.
- **No runtime execution of the factory.** `tsc` proves it compiles; it does not
  prove `new D1Database(env)` talks to D1. That needs vendor credentials and is
  a different pass.
- **Nothing equivalent for the other generated TypeScript.**
  `packages/wasm-bridge/src/generated.ts` and `src/identity.generated.ts` are
  typechecked incidentally, by being inside a package `pnpm typecheck` covers.
  The adapters factory was the one generated into a *product*, where no
  workspace typecheck reaches it.

# Prose is gated, not generated

**Date:** 2026-09-15
**Status:** implemented

---

## Context

This repository gates generated things well and hand-written things not at all.

| Gate | Catches |
|---|---|
| `fid derive --check` | an artifact that drifted from its declaration |
| `fid context --check` | a generated block that drifted from what it renders |
| `cargo test --test captures` | terminal output the binary no longer prints |
| `cargo test --test vectors` | two implementations of one rule disagreeing |

None of them catch a sentence. `AGENTS.md` said:

> **Adapters name contracts, not vendors.** Every contract currently implements
> only `none` — a real, working no-op.

That was true when written. It was false once `d1`, `r2`, `cloudflare`,
`turnstile`, `cloudflare-queues` and `supabase` landed — six vendors, across
several phases, with every gate above green the entire time. The `README.md`
had the same shape of failure with hand-typed member counts: "13 Rust members"
against a tree holding 15.

Both were found by a person reading, months late. In a repository whose entire
argument is that a hand-copied fact drifts and a gate is the only remedy, the
documentation *about* that argument was the least gated thing in it.

## The obvious fix does not work

Generate the prose. It fails, and pretending otherwise is how documentation
systems die.

A paragraph explaining *why* the adapter contract is shaped a certain way
cannot be derived from the contract. If it could be, it would carry no
information the code does not already carry, and there would be no reason to
read it. The judgment is the entire value. The parts of documentation that
*are* derivable are already derived — that is what `fid context` does, and the
adapter table it now renders is exactly the sentence above turned into a
derivation.

What was left is the residue: the part that needs a person or an agent to
think. That residue cannot be generated, so it has to be *watched*.

## Decision

**Derive the obligation to revisit the prose, not the prose.**

A prose block declares the source it describes:

```markdown
<!-- fid:describes crates/fiducial-cli/src/adapter.rs#pub static CONTRACTS -->
Every contract ships with `none` — a real, working no-op…
<!-- fid:end-describes -->
```

`docs/prose.lock` records a hash of that source as it stood when someone last
read the paragraph against it. `fid docs --check` fails when the source moved
and the paragraph did not, naming both. A person or agent re-reads it, fixes
what is now wrong, and runs `fid docs --accept` to record that they did.

The text stays hand-written. What stops is going stale *silently*.

### Why a symbol, not a whole file

`#pub static CONTRACTS` narrows the watch to one balanced block.

This is the difference between a gate and a nuisance. A whole-file hash fires
on every unrelated edit to a 200-line file, and a gate that cries wolf is worse
than no gate: it trains the reader to run `--accept` without reading, which is
the exact failure it was added to prevent. Both properties are tested —
`changing_the_described_symbol_fails_the_check` and
`an_unrelated_change_in_the_same_file_does_not_fire`.

The extractor is deliberately not a parser. It matches `{}`, `[]` and `()`,
respecting string literals, and stops when they balance. That covers a Rust
`static`/`struct`/`fn`, a TypeScript object, and a TOML array — every shape this
repository points prose at. A syntax tree per language is not worth it for a
staleness heuristic.

## Why not the alternatives

**A "last reviewed" date in front matter.** Nothing connects the date to
whether the underlying thing changed. It answers "when did someone look" and
the question is "has this become wrong", which are different, and the first is
satisfied by looking without reading.

**Fail on any diff touching both code and docs directories.** Backwards: it
punishes changing documentation, and passes for the entire class of change
where code moves and documentation is forgotten — the only case that matters.

**An LLM check in CI that reads prose against code.** Tempting and wrong as the
*gate*. It is non-deterministic, needs credentials and network in CI, and
produces a verdict nobody can reproduce locally. A hash is decidable and
offline. An agent is the right thing to *fix* a block once the gate names it,
which is what happens here — the gate points, the agent reads, the agent
accepts.

**Annotate everything.** The ceremony that kills the mechanism. Blocks are
opt-in per block, and a document with no markers is not a finding. Annotate the
paragraph that would be expensive to have wrong.

## Consequences

Promoting an adapter vendor now fails `fid docs --check` until someone re-reads
the paragraph about vendors — which is the outcome that was wanted six vendors
ago.

`--accept` is a separate verb from `--check` on purpose, and is never done
automatically on failure. The whole mechanism rests on somebody actually
reading, and a flag that accepts as a side effect of checking would make the
lock a record of nothing.

Two defects found and fixed before merge, both by the mechanism being used on
itself:

- The block count was derived from the map of successfully-hashed targets, so a
  document whose only block named a *renamed* symbol reported "no prose blocks
  declared" and exited zero. The gate disabled itself in precisely the case it
  was built to catch. `Report.total` counts blocks found rather than hashes
  produced. Caught by
  `a_renamed_symbol_is_reported_as_the_staleness_it_is`.
- The parser did not skip fenced code blocks, so the example in `AGENTS.md`
  showing *how to write a marker* registered as a real block — the file
  teaching the mechanism was permanently stale against it, and the spec you are
  reading made it worse by containing the same example. Fences are now tracked
  by token, so a ``` inside a ~~~ block does not close the wrong thing.

## What this deliberately does not do

- **No check that the prose is *correct*,** only that it has been read against
  the current source. Correctness is not decidable here; the claim is narrower
  and honest — nobody can now be unaware that the ground moved.
- **No coverage requirement.** Nothing measures what fraction of prose is
  annotated, and nothing should yet: a number would push toward annotating
  everything, which is the ceremony failure above.
- **Six blocks annotated, not every paragraph.** The ones chosen restate a
  registry, a list, or a layout rule — the shapes that have actually gone stale
  here before. More should be added as prose is written, not in a sweep.

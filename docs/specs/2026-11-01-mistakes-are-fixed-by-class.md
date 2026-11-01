# A mistake the gate misses is fixed by its class, not by its case

**Date:** 2026-11-01
**Status:** accepted
**Supersedes:** nothing

---

## Context

The paper's evaluation injects 20 realistic mistakes into a product and counts
how many the gate catches.

| Set | How it was written | First run | Ordinary CI |
|---|---|---|---|
| First set | hand-picked | 15/20; 20/20 after fixes | 1/20 |
| Held-out | written before its first run | 15/20 on 0.9.11 | 0/20 |
| Independent | written blind by a separate agent | 16/20 on 0.9.12 | 0/20 |

The 0.9.12 fixes were one rule per held-out miss, and the independent set
then missed four new cases. Each miss across the three sets falls into one of
a few classes, and six of the fourteen were the same class:

| Class | What happens | Misses |
|---|---|---|
| Shadow copy | a declared value typed again into hand-written code | N19, N20, I6, I20 and others |
| Gate removed | the CI step that runs the check deleted or silenced | I10 |
| Weak type | a field accepts any value of the right shape; an unknown key is ignored | N5, N6 |
| Unmodelled world | a rule from outside the product (a fab, a law, a datasheet) is nowhere in the data | N16, I7, I11, I12 |
| Untracked input | a pipeline reads a file nobody declared, so changing it never makes an output stale | — |

## Decision

Each class gets one mechanism, and no case gets its own rule:

1. **Shadow copies.** One scan by value (`src/shadow.rs`) runs on every
   `fid derive --check`, and flags two kinds of copy:
   - a distinctive string declared anywhere in `fiducial.toml` or a
     pipeline's TOML declaration (an email, a domain or URL, a colour, a
     multi-word name), found in any hand-written source file;
   - a number from a pipeline's TOML declaration (and a scale's inverse),
     found in a hand-written file that imports what that pipeline derives.

   Tests are exempt, since they state expected values on purpose.
   `fid: allow-value` marks a line that means its literal.
2. **Gate integrity.** `fid derive --check` and `fid doctor` fail unless a
   live workflow step runs `fid derive --check` in the product's directory,
   and does not end in `|| true`, set `continue-on-error: true` or `if:
   false` (`src/gate.rs`). A deleted scaffolded `ci.yml` fails too.
3. **Typed declarations.** Every `fiducial.toml` block that the CLI owns
   rejects unknown keys. A declaration is no longer repaired silently: a
   `[legal]` block missing a category its jurisdiction requires fails, and is
   not patched up in the output.
4. **Outside rules as data.** They live in profiles the derive reads, never in
   code paths:
   - `profiles/legal.toml`: pages and required cookie categories per
     jurisdiction, each with its source;
   - device profiles: a regulator's output voltage, and a logic input's
     V_IH as a fraction of its supply, checked against the rail declared on
     each net and against the supply of every profiled part driving it.
5. **Hermetic pipelines.** A `node` pipeline runs under Node's permission
   model. It may read only its script directory, its own pipeline file, its
   declared inputs and `node_modules`, and write only where its outputs go.
   An undeclared read fails, naming the file.

## Why not the alternatives

- **Another rule per miss.** That is what 0.9.12 did. It took the held-out
  set to 20/20 and did nothing for the next set. Five of the independent
  misses were in classes already seen.
- **Leave shadow copies to review.** A reviewer cannot tell `100` the scale
  from `100` the percentage either, unless they know which file imports the
  protocol. The scan uses exactly that fact.
- **A sandbox for every executor.** Built-in executors are the binary itself
  and already know their inputs. `shell` pipelines that do not run Node get
  no sandbox; that limit is stated below rather than hidden.

## Consequences

- A product with a typo'd key, a gate removed from CI, or a declaration the
  jurisdiction does not allow fails a gate it used to pass. This is a minor
  release, not a patch.
- Every class has one place to grow: a new device fact is a field in a
  profile, and a new jurisdiction is a block in `profiles/legal.toml`.
- The next evaluation is a fourth set written blind against these classes,
  not against cases.

## What this deliberately does not do

- It does not prove a declaration true. Every mechanism checks that facts
  agree with each other and with the data the platform holds. A wrong
  datasheet profile is still wrong everywhere at once.
- It does not sandbox `shell` pipelines that run Python or a binary.
- It does not find a shadow copy of a number in a file that imports nothing
  derived. That number could be anything, and saying otherwise would make the
  scan noise.

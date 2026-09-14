# Phase 21 — documentation that cannot lie

**Date:** 2026-09-14
**Status:** accepted

---

## Context

The platform was buildable and undocumented. There were excellent rustdoc
comments and eight design specs, but nothing that answered "what is this and why
would I use it" for someone who had not been in the room.

The request asked for guides "as if you're a student learning how to build
products", with screenshots.

## The screenshot problem

A screenshot or a pasted terminal block is a **derived artifact maintained by
memory**. It is correct on the day it is pasted and silently wrong from then on,
and the reader has no way to tell which they are looking at.

Writing one into the documentation of a platform whose entire thesis is *declare
once, derive the rest, and fail the build on staleness* would be indefensible.
The docs would be the least trustworthy artifact in the repository.

## Decision

Terminal output in the guides is **generated from the real binary and gated in
CI**, using the same mechanism `docs/protocol/vectors.json` already uses.

`crates/fiducial-cli/tests/captures.rs` runs a declared sequence of commands
against a freshly scaffolded product and records what they print.

```sh
FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures
```

CI runs it **without** that variable, so output that changed without
regeneration fails the build. Verified adversarially: tampering with one
character of a committed capture fails the test with the file named.

### Captures follow the guide's state, not a bare scaffold

The first harness ran every command against a fresh `fid new`. So `fid graph`
was captured as *"no pipelines declared"* and embedded into a guide section that
comes **after** `fid add eda` installs them.

That capture was real output, faithfully generated, and completely misleading —
introducing the exact failure mode generated documentation exists to remove.

Steps now run in order against one product, and a step may exist purely to
advance state (`fid add eda`, `fid derive`) without being captured. The
declaration is a walkthrough, not a list of commands.

### Captures are embedded, not linked

GitHub renders neither transclusion nor snippet syntax, and a guide whose
terminal output is a link nobody clicks shows nothing.

So the guides embed the output between HTML markers, and the block between them
is derived from the same `render()` the capture files come from. The guide is a
derivation; the capture is the declaration.

### Two tests, one source

Both the capture-file check and the guide-embedding check call `render()`
directly rather than one reading files the other writes. The first version had
the inliner read `docs/captures/`, which made it depend on another test having
already run — and cargo runs tests in parallel, so it read whatever happened to
be on disk. A test suite with an ordering dependency it does not declare is a
flaky suite that passes locally.

### Orphan detection

A third test asserts every declared capture is shown in some guide. An orphaned
capture is dead weight and a referenced-but-missing one renders as a broken
block; both are silent otherwise.

## The guides

| Guide | Answers |
|---|---|
| `start-here.md` | What is this, why is it shaped this way, what changes about how I work |
| `first-product.md` | Nothing → board → generated enclosure → CI gate, in ~20 minutes |
| `for-agents.md` | How to work here as an agent without breaking the property that makes it useful |
| `harvesting.md` | Getting the good parts out of a codebase you already built (Phase 19) |

`start-here.md` opens with a concrete failure rather than the thesis: one
connector position written down six times, and the fab run that discovers the
enclosure does not close. The abstraction is not persuasive before the reader has
felt the problem, and *"declare each fact once"* reads as a platitude until it is
attached to 500 units that do not fit.

`first-product.md` deliberately has the reader **break** `fid derive --check` by
editing a declaration and watching CI-equivalent failure, because the guarantee
is not "we generate things" — it is "an artifact that has drifted cannot reach
main", and that is only believable once seen failing.

## Consequences

- A change to any documented command's output fails CI until the guides are
  regenerated and the diff reviewed
- The guides cannot show output the CLI does not produce
- Adding a documented command means adding one line to `steps()`
- A new CI job, `docs`, runs the gate

## What this deliberately does not do

**No documentation site.** No mkdocs, no Docusaurus, no build step. Markdown in
the repository, rendered by GitHub, versioned with the code that produces it. A
docs site is a second place for documentation to live and a second thing to keep
in sync.

**No screenshots of the web surfaces.** Adding Playwright to the docs pipeline to
photograph the Next.js page was considered and declined: it is a heavy dependency
for a platform whose primary interface is a CLI. Revisit when a product's visual
surface is the thing being documented.

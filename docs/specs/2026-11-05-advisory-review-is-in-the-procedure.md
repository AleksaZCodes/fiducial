# Advisory review is part of the procedure, and switched off by a declared line

**Date:** 2026-11-05
**Status:** accepted

---

## Context

`fid advise` asks a model what no deterministic check can: a fact declared
twice, logic above the layer it could reach, a comment that says what rather
than why, a diff that does more than was asked. It existed, with a key command,
and nothing used it. Grepping the scaffolded `ci.yml`, the product `AGENTS.md`
and `docs/guides/for-agents.md` found no mention, so it ran only when someone
remembered, which for an agent means never.

It cannot simply be added to the gate. It exits 0 on findings and on a missing
key by design, a model's verdict varies, and `tests/determinism.rs` holds that
nothing a model says may reach a derived artifact or a gate's verdict.

## Decision

- Advisory review is **on by default** in a scaffolded product, and **off by
  declaration**: `[advise] enabled = false` in `fiducial.toml`. Absent means on,
  so a product that predates the block is not silently opted out.
- The switch is the escape hatch (principle 3) and is usable at any point in a
  product's life. `fid new --no-advise` only sets its starting value.
- It is a declared fact (principle 4), not an environment variable or a CI
  setting, so it can be reviewed, reverted and read by an agent.
- `fid advise` honours it itself: switched off, it says so, asks nothing and
  sends nothing. `fid derive`, `fid doctor` and the guard never read it.
- The scaffold asks for it in two places, neither a gate: a CI step on pull
  requests with `continue-on-error: true` and `OPENROUTER_API_KEY` from a
  secret, and a line in the product `AGENTS.md` and `for-agents.md` asking for it
  before a change is called done.

## Why not the alternatives

- **A required CI check.** A probabilistic verdict that can block a merge is a
  flaky gate, and flaky gates get disabled rather than diagnosed.
- **An environment variable or a `--no-advise` flag on every run.** A switch
  that lives in someone's shell is the state principle 4 exists to remove.
- **Branching the scaffold on the switch.** The CI step and the agent text are
  static, and the command decides. One template, no drift between the two.
- **Making `fid doctor` report the key.** `doctor` must be identical with and
  without a key (`tests/determinism.rs`).

## Consequences

- Every new product carries the step. With no `OPENROUTER_API_KEY` secret it
  prints that and exits 0, so adopting it costs nothing until a key is added.
- The CI step installs `@fiducial/advisor@0.3.0` with npm. That pin moves when
  the advisor package does.
- Existing products get none of it until `fid upgrade` merges the CI and
  `AGENTS.md` changes. `[advise]` absent already means on.

## Two models, in structural units

**Added the same day, before merge.** Running it on a real product (aleksa.works,
102 changed files) showed the original single request was the wrong shape: the
60KB budget filled in diff order and reported the remainder as "too large", which
dropped page components to make room for a generated module, and System One's
typed answers named no file or line. The user asked for the procedure to combine
System One with `anthropic/claude-haiku-5.5` for structural and reasoning review,
to enforce the repository's own rules, to catch documentation that was left
unchanged, and to hand the calling agent a list to check and fix.

- The change is split into **structural units**: a file; the hunks of a file
  larger than the budget, each keeping the file header; small neighbours packed
  by directory, then tiny units packed across directories. Nothing is dropped to
  fit, a unit is added instead. System One takes units of 40K characters and the
  reasoning model 120K, a few requests at a time.
- System One answers its typed questions for every unit and the answers are
  merged, keeping the strongest probability and the units that raised each.
- The reasoning model is given the product's own `AGENTS.md` and `MISSION.md`
  verbatim, a manifest of the repository's files (so "unchanged" is not read as
  "missing"), and the unit's diff. One further request gets no diff: the file
  list with line counts, and the documents the deterministic pass found that
  name something the change touched, new flags no document mentions, and a note
  when no document changed at all. Stale documentation is a cross-file fact, so
  it is judged there. `docs/specs/` is never a stale candidate: a decision is
  append-only, so the answer is a new dated decision.
- Skipped by name, and listed in the report: derived outputs (read from
  `pipelines/*.toml`), lockfiles and binaries, files `fiducial.lock` shows still
  match what the platform wrote, and capability skills and agent files, which
  `fid upgrade` owns. A finding about one is a finding the product cannot fix.
- The report ends with what the calling agent does with it: verify each item,
  fix it or say why it stands, update the documents the change made untrue, run
  it again. `--json` carries the same as `next_steps`. `--no-review` asks System
  One only; a TypeSafe-only key does the same, since the reasoning model is
  reached through OpenRouter.

Why not one very large request to the reasoning model alone: its window is
large, but the answer quality is not flat across it, and System One is the only
source of calibrated probabilities. Why not let the reasoning model edit: that
would make the thing that is wrong about 1 in 10 times the thing that writes.

Measured on the same product: the whole branch costs about $0.04 and finds 66
items, and the last commit alone costs about $0.004 and finds 13, which is the
usual case. Among them, the reviewer caught that the roadmap had moved Serbian
between releases and chosen OpenNext over the `deploy` capability without a
recorded decision (principle 1b), and a language-switch bullet left in the
design document after the locale was removed.

## What this deliberately does not do

- It does not read a key into CI or store one anywhere. The key is a secret the
  person adds.
- It does not make advice block anything, or edit any file.
- It does not promise precision. A first run over a large branch is noisy; the
  prompt caps each request at ten findings, asks for the most consequential
  first, and requires a "declared twice" finding to name both places.

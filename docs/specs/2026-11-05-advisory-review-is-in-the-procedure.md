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

## What this deliberately does not do

- It does not read a key into CI or store one anywhere. The key is a secret the
  person adds.
- It does not make advice block anything, or edit any file.

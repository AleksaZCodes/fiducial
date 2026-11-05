# `fid advise` runs before the commit, by the agent, and is not a CI step

**Date:** 2026-11-05
**Status:** accepted
**Supersedes:** the CI step, in part, of `2026-11-05-advisory-review-is-in-the-procedure.md`

---

## Context

The earlier decision put advisory review in two places: the agent instructions,
and a scaffolded CI step on pull requests, non-blocking, with the key as a
repository secret. It shipped (PR 112) and the product owner objected the same
day: a model's opinion does not belong in a pipeline, and the useful way to use
it is back and forth, before the commit.

Both halves of that are right, and the first was already implied by the earlier
decision's own reasoning. It kept the step out of the *gate* because a
probabilistic verdict that can block a merge is a flaky gate. Making it
non-blocking removed the flakiness from the gate and left the rest of the
problem in place: a job that runs after the push can only print a report, and the
report is read by nobody who can act on it in the loop that matters, which is the
agent that made the change fixing what it agrees with and asking again.

## Decision

- The scaffolded `ci.yml` has no `fid advise` step, no advisor install and no
  `OPENROUTER_API_KEY`, and no longer fetches full history for a base diff. A
  comment in the template records why, where someone would add it back.
- Advisory review stays in the procedure through the agent instructions
  (`AGENTS.md`, `docs/guides/for-agents.md`): run it before calling a change
  done, verify each item, fix it or say why it stands, run it again.
- The `[advise] enabled` switch is unchanged. It switches off the asking,
  wherever the asking is done from.
- Nothing about the key changes except where it lives: the environment of the
  machine or session the agent runs in (`OPENROUTER_API_KEY`, or `fid advise key
  set`). It is no longer a repository secret.

## Why not the alternatives

- **Keep the step but make it opt-in.** A step nobody turns on is the state
  `fid advise` was in before the earlier decision.
- **A pre-commit hook.** A hook runs the command but not the loop: nothing in it
  reads the report and acts. The agent does.

## Consequences

- Existing products that merged the earlier `ci.yml` keep the step until they
  remove it; `fid upgrade` merges the template change. A product that sets the
  `OPENROUTER_API_KEY` secret for it can delete the secret.
- Nothing runs `fid advise` unless the agent does, so the instruction in
  `AGENTS.md` is what makes it happen. That is the weaker guarantee, taken on
  purpose.
- The `@fiducial/advisor` version is no longer pinned in a workflow. A product
  that wants a fixed advisor pins `@fiducial/advisor` as a dependency, where
  `fid advise` already looks for it (`node_modules/@fiducial/advisor`).

## What this deliberately does not do

- It does not remove advisory review from the procedure, or make it optional.
- It does not make `fid advise` gate anything, here or anywhere.

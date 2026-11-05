# The sensor-stick example runs weekly and on demand, not on every pull request

**Date:** 2026-11-05
**Status:** accepted

---

## Context

`example (sensor-stick, end to end)` builds the whole worked product: routes and
design-rule-checks a board, builds and checks the case, writes fab files, and
builds the firmware and the page. It took about five minutes and 40 seconds on
the pull requests merged today, against roughly two minutes for every other job.
Because CI waits for the slowest job, it set the time of every Fiducial update,
including changes that cannot touch the example, such as a template comment or an
advisor package fix. The owner asked for it to be switched off for that reason.

## Decision

- The job runs on a weekly schedule (Mondays, 04:17 UTC) and on demand through
  `workflow_dispatch`. It does not run on pull requests or pushes to `main`.
- The mechanism is a job-level `if`, not removing the job. A job skipped by `if`
  reports as skipped, which GitHub counts as passing for a required status check,
  so a branch rule that names it keeps working without being edited.
- Run it by hand before a release, and after touching `examples/`,
  `crates/fiducial-eda`, `crates/fiducial-mesh` or the hardware capability.

## Why not the alternatives

- **Delete the job.** The example is the paper's and the README's evidence that
  the platform builds a real product. A check nobody runs lets it rot silently.
- **Run it only on changes to the paths it covers.** The honest path list is
  large and shifts with every capability, and a stale list is a check that quietly
  stops covering something. A weekly run is simple and cannot be wrong that way.

## Consequences

- A change that breaks the example now merges, and the break is found on the next
  scheduled run, up to a week later, or earlier if someone runs it. The cost of that
  is a bisect over a week of merges.
- The other jobs still cover the same crates and the hardware pipeline tests, so
  most breakage still shows up per pull request.

## What this deliberately does not do

- It does not remove or weaken the example, and it does not change what the job
  checks. It changes only when it runs.

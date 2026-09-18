# The remote is part of the product

**Date:** 2026-09-18
**Status:** accepted

---

## Context

Both this repository and `fon` declared `no-direct-main-push` in
`[guard] rules`. Neither had branch protection on GitHub. Both believed main was
protected. Neither was.

`guard.rs` already contains the argument against exactly this, one layer in:

> Returns `None` for a name with no implementation — which is a finding, not a
> shrug: a product listing it believes it is guarded and is not.

The guard rule *has* an implementation. It is a `PreToolUse` hook that fires on
`git push` from a machine with the Fiducial plugin installed. It does not fire
on another machine, on a CI job, or on anything holding a token. So the
declaration was true locally and false everywhere else, and nothing said so.

The working tree has been the unit of correctness here — `fiducial.lock`,
`fid derive --check`, `fid docs --check` all police files. The host the product
lives on was outside that boundary, and it holds settings that are as much a
part of the product as any file in it.

---

## Judgment: required checks are read from the workflows

A required status check whose name nothing produces blocks every pull request,
forever, with no error that names the cause. A hand-kept list of check names is
therefore not merely duplication — it is duplication whose failure mode is a
repository that cannot be merged into, triggered by renaming a job.

So `fid repo protect` parses `name:` out of `.github/workflows/*.yml`. Two
exclusions, both derived:

- **Release workflows.** They run on merge to the default branch, never on a
  pull request, so a required check by that name would never report.
- **Jobs declaring `continue-on-error: true`.** See below.

The parser is deliberately shallow — `name:` at a four-space indent inside
`jobs:`. A workflow it cannot read contributes nothing rather than contributing
a wrong name, and `--apply` prints everything it found before it writes.

## Judgment: advisory is declared by the job, not guessed from its name

The scaffolded `claude-review.yml` runs an Opus review on every PR. Made a
required check, it becomes a gate that depends on `ANTHROPIC_API_KEY` being
present and the model being willing — so the day the secret is missing or
rate-limited, a red review blocks a green branch.

Three ways to handle that were considered:

1. **Pattern-match the name.** Rejected: it cannot tell "Opus review" from
   "review coverage", and the rule would be invisible to whoever named the next
   job.
2. **A flag on the command.** Implemented as `--except`, but it is the worse
   mechanism — a flag someone has to remember, against a declaration the
   repository keeps. It survives as the escape hatch for a job a workflow cannot
   describe.
3. **Make the workflow say it.** `continue-on-error: true` already means
   "this job's failure is not the build's failure". Requiring it as a check
   contradicts the line directly, so a job carrying it is not required.

Option 3 is the one that generalises, and the line belongs on that job
regardless of branch protection: a review is an opinion. It was added to the
template.

## Judgment: zero required approvals

One approving review would lock a solo maintainer out of their own repository.
The rule being enforced is *go through a pull request* — which is what
`no-direct-main-push` says — not *find a second person*, which it does not.

`enforce_admins` is likewise false. The guard exists to prevent the reflexive
push, not to make the owner unable to act in an incident.

## Judgment: `fid doctor` reports, it does not fail

Three facts look identical from outside: an unprotected branch, a 403 from a
token without admin rights, and `gh` not being logged in. Only one of them means
the branch is open.

`doctor` distinguishes them and reports; it does not fail the run. Not every
product has a GitHub remote, protection needs admin rights `fid` cannot assume,
and a check that fails on "I could not tell" is a check people disable.

---

## Tools are a dependency

A capability can install every file it owns and still not work. `deploy` derives
a `wrangler.toml` that only `wrangler` can act on: correct, complete, unusable.
The gap announced itself as a command-not-found at deploy time rather than at
install time.

`requires_tools` in `capability.toml` is the declaration, alongside
`requires_adapters`, which it resembles: both say what must exist outside the
files for the files to mean anything. `fid doctor` reports what is absent.

Reported rather than failed, for the same reason as above: a missing tool is a
fact about *this machine*, and a CI runner legitimately has a different set from
a laptop.

---

## Consequences

- `fid repo protect [--apply] [--except JOB]` is a new command surface.
- `fid doctor` gains two checks and one new line of output; the
  `fid-doctor` capture changed by one line.
- `claude-review.yml` in the scaffold gains `continue-on-error: true`. Existing
  products get it through `fid upgrade`, or by adding the line.
- Nothing is applied automatically. `fid new` does not call `gh` — a scaffold
  that silently configures a remote is a scaffold that surprises someone, and
  the repository may not exist yet at that point.

## Not done

`fid new` does not protect the branch it just created, because at `fid new` time
there is no remote. The honest sequence is `fid new`, push, then
`fid repo protect --apply` — and `fid doctor` now says so when the second step
has not happened.

Nothing verifies that the protection *stays* applied. `doctor` reports it on
demand; no gate fails when someone turns it off.

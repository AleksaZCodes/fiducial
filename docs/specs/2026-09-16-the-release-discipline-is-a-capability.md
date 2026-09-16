# The release discipline is a capability, not a lesson

**Date:** 2026-09-16
**Status:** specified

---

## Context

On 2026-09-16 this repository learned how to make a release pipeline tell the
truth. It learned it the expensive way — five distinct failures, each found by
running a real release and reading what actually happened, none visible from a
green workflow:

| # | What broke | How it was found |
|---|---|---|
| 1 | The crates.io step was gated on `steps.changesets.outputs.published`, which reported `false` on runs that had just published to npm | The step said **skipped** on two consecutive successful releases |
| 2 | `changeset publish` printed `Successfully published: @fiducial/adapters@0.2.0` for a package that stayed at 0.1.0 | `time.modified` on the registry was unchanged |
| 3 | Release PR CI sat in `action_required` forever once the branch ruleset required status checks | The Release PR became unmergeable |
| 4 | A **queued** release run was cancelled one second after an unrelated merge queued behind it, publishing nothing and running no step | Compared run timestamps after npm 404'd on a version main declared |
| 5 | The verify step failed a release that had worked, asking npm twelve seconds after a write that took ninety-six seconds to propagate | Compared the publish timestamp to `time.modified` |

Every one of those findings now lives in files only *this* repository has:
`release.yml`, three scripts under `scripts/`, three tests in
`workspace_hygiene.rs`, a PR template, a CLA exemption list.

A product scaffolded by `fid new` gets `ci.yml` and `claude-review.yml`. It gets
nothing about releasing, publishing, verifying a publish, or repository
hygiene. So the next product re-derives all of it — by hitting the same five
failures, in the same order, on a real release, at whatever moment is least
convenient.

**That is the exact failure this platform exists to prevent, turned on
itself.** Principle 7 says a fix that cannot propagate is only half-finished.
These fixes cannot propagate at all.

## The two obvious fixes, and why neither works

**"Write it down in `AGENTS.md`."** A paragraph does not push a tag. Worse, a
summary of a mechanism is a second declaration of it, and it is the copy that
goes stale — the argument this repository already makes about `SHIPPED.md`
stating what is next, and about `CLAUDE.md` restating `AGENTS.md`. The lesson
would be documented and still absent from every product.

**"Copy `release.yml` into the scaffold."** Most products should not have it.
This repository publishes fourteen npm packages and fifteen crates; the next
product might publish nothing, or a container image, or only deploy a Worker. A
scaffolded workflow that assumes two specific registries is wrong for most
products, and a wrong template gets edited — at which point every product holds
a divergent copy of a thing the platform was supposed to own.

## What actually generalizes

The mistake is treating this as one body of knowledge. It is three, and they
want three different mechanisms.

| Bucket | Example | Portable? |
|---|---|---|
| **Principle** | "a publish step exiting 0 is not evidence anything was published" | Everywhere. True of every registry, deploy target and release tool |
| **Pattern** | verify against the authority rather than the claim; a serialized workflow needs a schedule because queued runs get evicted; registries are read-after-write eventual | Everywhere a product releases *something*, whatever that something is |
| **Vendor mechanics** | `curl registry.npmjs.org/…`, `cargo publish --package`, `changesets/action@v2.1.2` | Only to a product publishing to that exact target |

And one bucket that looks portable and is not: `docs/release/legacy-untagged.txt`
is an incident record, not a mechanism. It belongs to this repository alone.

Repository hygiene is a fourth thing and the simplest: a PR template, a
Conventional Commits gate, a CLA sign-off gate with its exemption list, and a
branch lifecycle. None of it assumes a registry, so all of it is portable
without a declaration to configure it.

## The shape

Three mechanisms, matching the three buckets:

**Principles go to `MISSION.md`**, which is where principles are authored and
where a scaffolded product already receives them.

**Portable artifacts go to `SCAFFOLD_FILES`** in `templates.rs`. That list is
read by `fid new` *and* `fid upgrade`, which is the whole reason it is a list
rather than a sequence of calls — adding to it reaches products that already
exist. This is where the PR template, the commit-convention gate and the branch
lifecycle belong.

**Vendor mechanics go to a `release` capability**, because a capability is the
platform's answer to "this applies to some products and not others". It carries
a declaration naming what the product publishes, so a product that publishes
nothing installs nothing, and a product that publishes to npm gets the npm
verification without also getting cargo's.

## What the capability must carry that is not code

**The evidence, not just the rule.** A settle window on a 404 looks like
superstition. A schedule on a workflow that already triggers on merge looks
redundant. A verification step that re-asks a registry the workflow just wrote
to looks paranoid. Each of them will be deleted by the next person to read the
file quickly and tidy it — which is precisely how the `published` gate got
written in the first place, and how the tag push sat broken for twelve
versions.

So the capability's `SKILL.md` carries the incident with its timestamps:

> verify asked npm at 16:30:10Z and got a 404. npm began serving that version
> at 16:31:34Z. The publish at 16:29:58Z was fine.

A rule with its evidence attached survives a tidy-up. A rule without it does
not. This is the same reason `commit_hygiene.rs` opens by naming the asymmetry
it closes, and why `publish-crates.sh` explains the 429 rather than just
sleeping.

## Consequences

A product scaffolded after this exists starts with the repository hygiene that
took this repository a day to work out, and installs release verification only
if it releases something. A product scaffolded *before* it exists picks up the
portable half on its next `fid upgrade`, because `SCAFFOLD_FILES` is what
`upgrade` reads.

The five failures above stop being this repository's biography and become
something the platform knows.

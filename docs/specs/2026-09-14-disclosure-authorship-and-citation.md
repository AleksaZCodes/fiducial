# Disclosure, authorship, and citation — the state of play

**Date:** 2026-09-14
**Status:** accepted as a factual record
**Audience:** the author, and any attorney he takes this to.

> **This is not legal advice.** It is a written record of what has happened and
> when, assembled so that a qualified attorney does not have to reconstruct it
> from a chat log or a commit history. Every legal conclusion below should be
> confirmed.

---

## Why this document exists

`IP-POLICY.md` states a rule:

> A provisional is filed before the commit that makes any novel thing public.

That rule was not followed — the repository was public from Phase 0. This
document records that plainly, because an undocumented mistake gets rediscovered
and re-panicked-about, and because the author intends to write an IEEE student
conference paper about this work and needs to know where he actually stands.

---

## The four rights, kept separate

These get conflated constantly. They are independent.

| Right | Status | Affected by publishing? |
|---|---|---|
| **Copyright** | Owned by Aleksa Zdravkovic | **No.** Ownership is automatic on creation. |
| **Licence** | MIT — a grant of *use* to others | It *is* the publishing act |
| **Trademark** | "Fiducial" the name and mark, fully retained | **No.** Separate from the code licence. |
| **Patent** | See below | **Yes — severely.** |

**Copyright is not surrendered by open-sourcing.** MIT grants others permission
to use, copy and modify. It transfers nothing. The author remains the copyright
holder and may dual-licence his own work later — which is precisely why
`IP-POLICY.md` chose MIT over Apache-2.0, since Apache-2.0 contains an express
patent grant.

---

## Patents — the honest position

**Publishing source code publicly is "public disclosure."** It becomes prior art
against the discloser's own later application.

| Jurisdiction | Grace period after disclosure |
|---|---|
| United States | 12 months to file |
| EPO and most of the world | **None.** Disclosure destroys novelty permanently. |

**Consequence for what is already public:** for anything disclosed in the public
`fiducial` repository, non-US patent rights are most likely already unavailable,
and any US filing runs against a clock that started at first disclosure.

**What is probably preserved, and why it may matter more than it sounds:**

`IP-POLICY.md` draws a two-tier line, and the private tier has been honoured:

> Product repos, domain logic, novel protocols, novel algorithms, and hardware
> designs live in private repositories and are never published.

So if genuine novelty exists, it is most likely in the private tier — `fon`'s
hardware, radio policy and security posture; ROP's domain logic — not in the
public platform. The platform is integration work, and integration work is
rarely patentable anyway.

**What to ask an attorney:**

1. Is anything in the public platform plausibly patentable, given software
   patent limits in the EU and the `Alice` standard in the US?
2. Does the US 12-month clock start from first disclosure, and has it run?
3. Does publishing an IEEE paper constitute a further disclosure with
   consequences separate from the code already being public?
4. Is the two-tier boundary in `IP-POLICY.md` drawn where an attorney would draw
   it?

**Practical read, pending that advice:** the realistic protection here is
copyright, trademark and *first-mover authorship*, not patents. The paper
strengthens exactly those.

---

## The CLA gap — the most time-sensitive item

`IP-POLICY.md` requires one:

> No external contribution is merged before a CLA is in place.

**There is no CLA.** The repository is public and able to accept pull requests
right now.

**Why it matters, plainly:** a contributor owns the copyright on the lines they
write. Without an agreement granting those rights, the project can never be
re-licensed — not dual-licensed, not relicensed, not offered commercially —
without tracking down every contributor for permission. One accepted pull
request from a stranger permanently constrains the licensing options.

The exposure is currently zero contributions, which is the best possible moment
to fix it.

---

## Citation and academic credit

For a paper, the goal is: **maximally useful to the community, with authorship
unambiguous and permanent.** MIT already does the first. Three things do the
second, and none exist yet.

### `CITATION.cff`

A standard machine-readable citation file. GitHub renders a **"Cite this
repository"** button from it and offers BibTeX and APA. Without it, people cite
a URL, inconsistently or not at all.

### A DOI, via Zenodo

A GitHub release archived to Zenodo mints a permanent Digital Object Identifier.

Its value is exactly as the author identified: **independence from
infrastructure that dies.** A URL dies when a domain lapses. An email dies when
an account closes. A DOI resolves forever and is what a reviewer expects to see.
Zenodo issues a *concept DOI* for the project plus a *version DOI* per release —
so one can cite "the project" or "the exact version measured."

### `AUTHORS` / contributor record

An explicit statement of who wrote what, which becomes load-bearing the moment
there is more than one contributor.

---

## Framing the contribution in the paper

The honest claim — and the strongest one, because it is defensible:

> The contribution is not a new algorithm. It is an **architecture and a working
> system** for eliminating cross-domain fact duplication in single-operator
> product development, demonstrated across electronics, firmware, protocol, web
> and mechanical domains, with machine-checked guarantees that derived artifacts
> cannot silently drift from their declarations.

What makes that publishable rather than a tools paper:

- **A stated, falsifiable mechanism.** Not "we generate files" but "an artifact
  that has drifted from its declaration cannot reach the main branch," enforced
  by `fid derive --check` in CI.
- **Cross-domain evidence.** One board declaration deriving a printable
  enclosure, TypeScript types and a protocol — spanning EDA, mechanical CAD and
  web, which is unusual in the literature.
- **Adversarial verification, not assertion.** Conformance vectors verified by
  flipping a CRC polynomial bit; meshes ray-probed on shipped STL bytes;
  freshness gates verified by tampering. This is the section reviewers respect.
- **A negative result worth reporting.** The Phase 18 audit found the platform
  violating its own central rule in nine places while every test passed. *A
  green build is not evidence of consistency* is a genuine, generalizable
  finding — and self-criticism in a systems paper reads as rigour.
- **Honest anti-goals.** Stating what was deliberately not built, and why,
  distinguishes engineering from advocacy.

**What to avoid claiming:** novelty over existing build systems, monorepo tools
or code generators in general. The novelty is the *cross-domain* application and
the *staleness-as-build-failure* guarantee, not generation itself.

---

## Actions, ordered by urgency

| # | Action | Why now |
|---|---|---|
| 1 | Add `CLA.md` | Zero contributions today; one PR changes that permanently |
| 2 | Add `CITATION.cff` | Prerequisite for being cited correctly at all |
| 3 | Tag a release, connect Zenodo, obtain a DOI | Needed before the paper is submitted |
| 4 | Add `CONTRIBUTING.md`, `SECURITY.md`, `AUTHORS` | Standard, cheap, expected of a public project |
| 5 | Take this document to an attorney | Before the paper is submitted, not after |
| 6 | Record the outcome as a new dated decision | Decisions are appended, never edited |

Items 1–4 are the roadmap's **open-source bootstrap** (item 31), and Fiducial is
its own first customer: it currently has none of these files.

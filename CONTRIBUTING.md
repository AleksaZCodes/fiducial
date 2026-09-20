# Contributing

Thank you for considering it. This page is short on purpose — most of what you
need is already written somewhere, and a second copy of it here is the exact
failure mode this project exists to prevent.

## Where this project is right now

**Issues, bug reports and questions are genuinely welcome. Pull requests from
outside contributors are not currently being merged.**

That is a statement about the project's stage, not about the quality of anyone's
work, and it is worth being concrete about why:

- **Patents.** Novel work here — protocols, algorithms, hardware designs — is
  headed for provisional filings, and most of the world allows no grace period
  after disclosure (see [`IP-POLICY.md`](IP-POLICY.md)). Outside code in the
  tree makes the provenance of a claimed invention a question that has to be
  answered rather than assumed.
- **Relicensing.** Every outside commit needs a CLA sign-off, permanently, or
  the project loses the ability to be relicensed without tracking down each
  contributor individually. `CLA.md` explains it in full. That is a real
  obligation to take on, and taking it on casually is worse than declining.
- **It is one person.** Reviewing a patch to the standard the gates in this repo
  demand costs more than writing it. A queue of well-meant PRs is a queue.

**So: if you find something wrong, please open an issue.** A report is worth as
much as a patch here — arguably more, because the fix is usually the easy half
and *noticing* is the hard one. Reported bugs get fixed with a
`Reported-by:` trailer naming you in the commit, which is real credit in the
history and carries none of the above complications.

If you would like to work on the platform itself rather than report to it, say
so in an issue and we can talk about it directly.

The rest of this page describes the standard a change is held to, and stands
regardless — the maintainer is bound by all of it too.

## Read first

**[`AGENTS.md`](AGENTS.md)** is the working context for this repository: the
layout, the build commands, the freshness gates, and what not to do. It is
written to the portable `AGENTS.md` convention, so it is the same file a
Codex, Cursor or Copilot Workspace session reads — and the same one you should.

**[`MISSION.md`](MISSION.md)** is why any of it is shaped this way. A change
that fights the principles will not survive review, and reading them first is
cheaper than finding out in a PR.

## Before you write code

**Open an issue.** There are templates for a
[new capability](.github/ISSUE_TEMPLATE/capability.md) and for
[drift](.github/ISSUE_TEMPLATE/drift.md).

This used to say "a bug fix or a typo needs no issue", which invited the pull
requests the section above now declines — an outside contributor followed that
sentence exactly and in good faith, and the contradiction was the project's
fault rather than theirs. An issue is the path for a bug of any size.

The bar for adding to the platform is deliberately high, and it is stated in
`MISSION.md` as anti-goal 2: *nothing is added speculatively; a capability
enters the platform when a real product needs it, and is generalized when a
second one does.* If you are proposing a capability, the most useful thing you
can put in the issue is **which real product needed it.**

## The rules that a test enforces

These are not style preferences. Each one fails CI.

| Rule | Enforced by |
|---|---|
| Commit subjects are Conventional Commits (`feat(scope): subject`) | `cargo test -p fiducial-cli --test commit_hygiene` — only on commits not yet on `main`, the ones you can still amend |
| Every commit from a non-maintainer carries `Signed-off-by:` matching its author | the same suite, against `.github/cla-exempt.txt` |
| No derived artifact has drifted from its declaration | `fid derive --check`, and the gates listed in `[freshness]` in `fiducial.toml` |
| Generated files are not hand-edited | the same gates — they will simply overwrite you |
| Terminal output shown in the guides matches the real binary | `cargo test -p fiducial-cli --test captures` |
| Hand-written prose still matches the source it describes | `fid docs --check` |

On that last one: `fid docs --accept` means **"I have read this paragraph
against its source."** Running it to turn a red build green, without reading,
is the one action that makes the mechanism worthless. It is a separate command
from `--check` for exactly that reason.

## The CLA

By [`IP-POLICY.md`](IP-POLICY.md) rule 3, no outside contribution is merged
without one. Practically, that means **`git commit -s`** on every commit,
which adds the `Signed-off-by:` trailer certifying that you agree to
[`CLA.md`](CLA.md).

It is not a formality and it is not about ownership of your work — you keep
that. It is that a contribution merged without it permanently removes this
project's ability to be relicensed without finding you again, individually,
forever. `CLA.md` explains the reasoning in full.

Maintainers and automation are listed in `.github/cla-exempt.txt`, which is
the only place that decides who is exempt.

## Opening the pull request

[The PR template](.github/pull_request_template.md) carries the checklist,
including the CLA line and the **disclosure checkpoint** — whether the change
discloses a novel invention. That checkpoint is not decoration: public
disclosure is prior art against the project's own future filings, and most of
the world allows no grace period at all. If you are unsure, answer as though
it were "yes" and ask before merging.

**A judgment that could not be derived belongs in `docs/specs/`** as a dated
decision record, using [the template](docs/specs/_template/decision.md).
Decision records are append-only: superseding one means writing a new dated
file that says so, never editing the reasoning in the old one. Being able to
see what was believed at the time is most of their value.

## Security

Do not report vulnerabilities through issues or pull requests. See
[`SECURITY.md`](SECURITY.md).

## Conduct

[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) applies to every space this project
uses.

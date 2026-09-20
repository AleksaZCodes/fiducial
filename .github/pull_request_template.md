<!--
────────────────────────────────────────────────────────────────────────────────
BEFORE YOU FILL THIS IN — are you an outside contributor?

Pull requests from outside contributors are not currently being merged. This is
about the project's stage, not your work: novel work here is headed for patent
filings where most of the world allows no grace period after disclosure, every
outside commit carries a permanent CLA obligation, and the project is one
person. CONTRIBUTING.md states it in full.

Please open an ISSUE instead. A report is worth as much as a patch here — the
fix is usually the easy half and noticing is the hard one — and reported bugs
are fixed with a `Reported-by:` trailer naming you in the commit.

Maintainers listed in .github/cla-exempt.txt: carry on.
────────────────────────────────────────────────────────────────────────────────
-->

## What changed, and why

<!-- The why matters more than the what — the diff already says what. -->

## Checklist

- [ ] `fid doctor` clean
- [ ] `fid derive --check` passes (no artifact drifted from its declaration)
- [ ] New behaviour has a test that would fail without it
- [ ] A judgment that could not be derived is recorded in `docs/specs/`
- [ ] No generated artifact edited by hand
- [ ] No value typed a second time — it has a declaration

## Contributor License Agreement

> Required by [`IP-POLICY.md`](../IP-POLICY.md) rule 3. Without it, this
> contribution can never be relicensed without tracking you down individually.

- [ ] Every commit carries `Signed-off-by:` matching its author
      (`git commit -s`), certifying agreement with [`CLA.md`](../CLA.md)
- [ ] Not applicable — I am a maintainer listed in `.github/cla-exempt.txt`

## Disclosure checkpoint

> Required by [`IP-POLICY.md`](../IP-POLICY.md). Public disclosure is prior art
> against your own future application: the US allows a 12-month grace period and
> **the EPO and most of the world allow none.**

**Does this PR introduce or disclose a novel invention?**

- [ ] **No** — integration, refactoring, documentation, or an application of
      known technique
- [ ] **Yes** — a provisional has been filed. Filing date and number:
      `____________`

<!--
If you are unsure, answer as if it were "yes" and ask before merging. This
checkpoint existed as prose in IP-POLICY.md with nothing behind it, so PRs
merged without answering it. Reversing that is the point of this template.
-->

## What changed, and why

<!-- The why matters more than the what — the diff already says what. -->

## Checklist

- [ ] `fid doctor` clean
- [ ] `fid derive --check` passes (no artifact drifted from its declaration)
- [ ] New behaviour has a test that would fail without it
- [ ] A judgment that could not be derived is recorded in `docs/specs/`
- [ ] No generated artifact edited by hand
- [ ] No value typed a second time — it has a declaration

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

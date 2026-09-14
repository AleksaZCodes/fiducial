---
name: Drift
about: Something derived no longer matches its declaration
labels: drift
---

**The declaration** — which file states the fact?

**The artifact** — what was derived from it, and how does it disagree?

**How it was found** — CI, `fid doctor`, `fid dash`, or by eye?

> If it was found by eye, that is the more important half of the report: a drift
> a human noticed before a gate did means the gate is missing, and the fix is the
> gate rather than the artifact.

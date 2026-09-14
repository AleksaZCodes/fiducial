---
name: Capability request
about: Something the platform should be able to do
labels: capability
---

**Which real product needs this, and for what?**

> `MISSION.md` anti-goal 2: nothing is added speculatively. A capability enters
> when a real product needs it, and is **generalized when a second one does.**
> "It would be useful" is not an answer to this question.

**Is this the first product to need it, or the second?**

- [ ] First — build it *in the product*, and open this again when a second needs it
- [ ] Second — name the first:  `____________`

**Which kind is it?** (see `docs/specs/2026-09-14-capability-taxonomy.md`)

- [ ] **Declaration** — typed facts, inert
- [ ] **Pipeline** — reads declarations, produces artifacts, gated by `derive --check`
- [ ] **Adapter** — a swappable implementation behind a fixed contract
- [ ] **Capability** — a bundle of the above

**Cost of delay** (principle 5c) — what does waiting cost?

- [ ] **Debt-accruing** — the cost grows while we wait
- [ ] **Multiplying** — it makes later work cheaper
- [ ] **Terminal** — it costs the same whenever

# IP Policy

**Not legal advice. Confirm with a patent attorney before any public disclosure.**

---

## Two tiers

| Tier | Contents | Status |
| --- | --- | --- |
| **Public** | CLI, guardrails, propagation, tokens, harness, templates, protocol primitives | Published, **MIT** |
| **Private** | Product repos, domain logic, novel protocols/algorithms, hardware designs | Never published |

The architecture draws this line for modularity reasons (`MISSION.md` anti-goals). That same
boundary is the IP boundary — business logic stays in the product, never in the platform.

---

## Rules

**1 · File a provisional before any public disclosure.**

Public disclosure is prior art against you. The US allows a 12-month grace period; **the EPO
and most of the world allow none** — publish before filing and novelty is destroyed permanently.

**Rule: a provisional is filed before the commit that makes any novel thing public.**

**2 · Publishing does not surrender copyright.**

You remain the copyright owner and may dual-license your own work later. MIT was chosen partly
because Apache-2.0 contains an express patent grant — if retaining patent rights matters,
Apache is the worse choice.

**3 · CLA before accepting any outside contribution.**

Without a Contributor License Agreement, dual-licensing the project later becomes impossible.
No external contribution is merged before a CLA is in place.

**4 · Trademark is handled separately from the code license.**

Code may be MIT; the Fiducial name and mark remain fully owned. Do not conflate the two.

---

## PR checkpoint

Every PR that makes a new thing public carries an explicit answer to:

> Does this PR introduce or disclose a novel invention?
> If yes: has a provisional been filed? Record the filing date and number here.

---

## The private tier is never published

Product repos, domain logic, novel protocols, novel algorithms, and hardware designs live in
private repositories and are never published — not under MIT, not under any other license.
The platform is useful without them. They are the reason the platform exists.

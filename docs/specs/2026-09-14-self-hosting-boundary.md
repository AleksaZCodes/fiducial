# How much Fiducial should use Fiducial

**Date:** 2026-09-14
**Status:** accepted

---

## The measurement

Mechanically, almost none.

| | State |
|---|---|
| `fiducial.toml` | **absent** — so `fid dash`, `fid doctor` and `fid derive` cannot run in this repository at all |
| Guard hook | **not activated** — only a personal `settings.local.json`, no committed activation block |
| Generated artifacts | three, real: `docs/protocol/vectors.json`, `packages/wasm-bridge/src/generated.ts`, `docs/captures/` |
| How they are gated | **two bespoke Rust test files** re-implementing the `fid derive --check` pattern by hand |

So the headline is: **the platform re-implements its own central mechanism twice
rather than using it.** The philosophy is followed closely; the tooling is not
used at all.

## Is recursion possible?

Yes. Compilers do it — rustc and the Go toolchain both self-host. It costs a
**bootstrap**: `fid` is built by `cargo` from this repository, so if this
repository's artifacts were produced by `fid derive`, you need a `fid` to build
the repository that produces `fid`.

That is solvable the standard way — a released stage-0 `fid` derives, then the
new one is built and can re-derive to check it agrees. But it brings three
things that are not free:

1. A fresh clone cannot build without first fetching a released binary.
2. Version skew: *which* `fid` produced the committed artifacts — the one in the
   tree or the released one? That question has no good answer during a change to
   the derive logic itself.
3. CI has to hold both.

## The decision: three levels, and only two are adopted

### Level 1 — dogfood the *practices*. Already done. Keep it.

Declare once, derive, gate staleness in CI. That is followed throughout.

**And the bespoke gates are correct, not lazy.** `docs/protocol/vectors.json` and
`docs/captures/` are gated by Rust tests rather than by `fid derive` on purpose:
those gates must fire **even when `fid` is broken**. A freshness check that runs
*through* the tool it is checking has a blind spot exactly where it matters — the
tool breaking is the case you most need caught.

This is the general rule: **a gate must not depend on the thing it gates.**

### Level 2 — dogfood the *tools*. Adopt.

Add a `fiducial.toml` so `fid dash` and `fid doctor` run here, and commit the
guard activation block.

No bootstrap problem: these commands *read* the repository, they do not produce
what builds it. The cost is one config file; the benefit is that the platform's
own dashboard reports on the platform, and the guard that products get is the
guard we work under. Today neither is true, which means every dogfooding signal
`fid dash` was built to give is unavailable in the repository that built it.

Fiducial is also the natural **first customer** for two roadmap items:
open-source bootstrap (it has none of `CITATION.cff`, `CLA.md`,
`CONTRIBUTING.md`, `SECURITY.md`) and context sync (its agent-context files have
drifted twice already).

### Level 3 — dogfood the *build*. Decline, for now.

Making this repository's artifacts flow through `fid derive` buys consistency and
costs a bootstrap, and for the two artifacts that matter most it would actively
weaken them per level 1.

Revisit only if a derived artifact appears here whose gate does **not** need to
be independent of `fid`.

## The risk worth naming

**If Fiducial's only customer is Fiducial, the platform gets shaped by the needs
of a platform rather than a product.**

This is a cross-domain system for electronics, firmware, protocols, mechanical
CAD and web. Its only user being a Rust CLI repository would bend it toward
CLI-shaped features while the EDA and mesh paths quietly rot from disuse — and
the tests would stay green throughout, because nothing exercises them in anger.

`MISSION.md` is explicit that shipping products is the point and the platform is
the residue. So self-hosting is a **source of signal, not a substitute for a
real product.** Adopting level 2 is worth it precisely because it is cheap;
treating it as evidence the platform works would be a mistake.

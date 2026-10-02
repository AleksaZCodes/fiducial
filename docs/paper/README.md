# The paper — working notes

**Working title:** *Fiducial: Build any robust multidisciplinary product with
AI agents*

**Author:** Aleksa Zdravković (ORCID 0009-0004-1535-1839)

The title is the thesis; what must be true for it, and the plan that makes
it true, is `docs/specs/2026-10-02-making-the-thesis-true.md`.

## Thesis

Fiducial's own (`MISSION.md`): **declare each fact once; derive every
artifact from it.** In a cross-domain product the same fact is written down
many times, the copies drift, and drift is found late. One typed
declaration, derived artifacts and a gate that fails on disagreement remove
that class of error.

The paper's claim, narrower and evidenced: *a single person working with AI
agents can carry a cross-domain product to a manufacturable, checked state
when every fact is declared once, every artifact is derived and gated, and
hard problems are delegated to proven engines behind fixed contracts — so
the system grows by declarations, not by rules.*

## Evidence available in this repository

- The gates: `fid derive --check`, input tracking, CI on every commit.
- The hardware capability end to end on a reference product (private — cite
  as such, show no design detail): one declaration → case, routed DRC-clean
  board, schematic, BOM, checked assembly.
- Placement by CP-SAT: every rule an instance of eight generic kinds;
  contradictions named as a minimal set; 13 % shorter connections than the
  hand-written search, then 10 % more with a warm start; multi-worker CP-SAT
  found non-deterministic, replaced by a deterministic single-worker LNS
  (`docs/specs/2026-10-01-hardware-is-solved-by-engines-not-rules.md`).
- The request ledger and retrospective in that spec: where each change
  landed (platform vs product) and what was slow or wrong.
- A public example product built in this repository (`examples/sensor-stick/`,
  G3 of the plan) — the paper's worked example.

## Limits to state

N = 1 product; floor placement still a bounded search; autorouting not
deterministic; power-circuit values computed, not simulated; nothing yet
manufactured and tested.

## Citing the software

Cite the **version DOI** of the release the paper describes (see
`docs/specs/2026-10-02-one-release-per-platform-version.md`): connect Zenodo,
merge the release that carries the hardware work (0.9.0), and the `v0.9.0`
release mints it.

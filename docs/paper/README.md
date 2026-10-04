# The paper — working notes

**Working title:** *Fiducial: Build any robust multidisciplinary product with
AI agents*

**Author:** Aleksa Zdravković (ORCID 0009-0004-1535-1839)

<!-- fid:describes thesis.toml -->
The title is Fiducial's thesis — the platform's standing claim, declared in
`thesis.toml`, which the paper argues; it is not the paper's own. What must
be true for it, and the plan that made it true, is
`docs/specs/2026-10-02-making-the-thesis-true.md`.
<!-- fid:end-describes -->

## Mechanism

How the thesis is met (`MISSION.md`): **declare each fact once; derive every
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

## Discussion: judgment stays human; fiducial bounds the randomness

The author's position, for the discussion section:

- **Better models move the line, not the need.** Even with the best models,
  a person still questions the output, applies judgment and tests it. As
  frontier models improve, more is offloaded and the checking gets lighter,
  but a probabilistic system cannot be guaranteed correct. Its output also
  cannot be guaranteed to match the product's constraints and wants. Even a
  deterministic tool is only right about what it was told.
- **What fiducial is for.** It is the layer that reduces that randomness. It
  is a deterministic way to build every part of a product in one place, with
  one shared context. Declarations are the constraints, derivations are
  repeatable, and gates refuse drift.
- **Product-oriented, not codebase-oriented.** Scattered codebases, each
  with an agent that knows only its own corner, give incoherent products.
  One declaration read by every agent, across software, hardware and
  everything between, makes a product designed as a whole.
- **Evidence from this work.** A gate caught every disagreement, and the
  person still had to judge what the gates could not:
  - a catalogue that changed format overnight;
  - a substring match that took "±0.1 %" for "1 %";
  - a sensor isolated from the board's heat;
  - a case its board did not fit.

## Limits to state

N = 2 products (1 public); autorouting not
deterministic; set points solved for DC only, nothing simulated in time; nothing yet
manufactured and tested.

## Citing the software

Cite the **version DOI** of the release the paper describes (see
`docs/specs/2026-10-02-one-release-per-platform-version.md`): connect Zenodo,
merge the release that carries the hardware work (0.9.0), and the `v0.9.0`
release mints it.

## How the paper gets written (owner's plan, 2026-10-04)

Not before Fiducial is finished and makes sense as a whole. Then the owner
provides:
- the conference's `.docx` paper template;
- last year's best-paper award winner, written by the owner.

From those: the owner's writing style, what made that paper succeed, and how
its images and illustrations work. Then, together: how best to present,
sell and frame Fiducial. The draft written on 2026-10-02 was removed; start
from these notes, not from it.

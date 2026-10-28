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
`docs/specs/2026-10-02-one-release-per-platform-version.md`). Zenodo has
been connected since `v0.9.0`, so every release from it on has a version
DOI; the hardware work the paper describes is complete from `v0.9.2` (the
solvers, set-point checks, prices and the cut board are in `v0.9.1`, and
`v0.9.2` makes `--check` regenerate the hardware outputs).

## The paper

For IEEESTEC 2026, on the conference template, in the voice of the author's
IEEESTEC 2025 paper (Outreach). It cites release 0.9.11. After review, its
title is *Fiducial: Keeping Multidisciplinary Products Built with AI Agents
Consistent*: the reviewer found "Build Any Robust …" (the platform's thesis,
`thesis.toml`) too broad for two products, and the paper now claims
consistency between artifacts, not correctness of the declared facts.
`response-to-review.md` answers the review point by point.

- `Fiducial_Zdravkovic_Aleksa.docx` / `.pdf` — the paper.
- `markers/run.py` — the injected-mistake test. Two sets of 20 mistakes,
  each applied to a clean copy of a product with the gates run in CI's
  order:
  - the first set (with 5 controls): 15/20 on 0.9.4, 20/20 on 0.9.11;
    it was used to improve the platform;
  - the held-out set (N1–N20), written after 0.9.11 and committed before
    its first run (`bbaa1b0`): 15/20, reported with no rule added first.

  `--baseline` runs either set with derive and `--check` removed, i.e.
  ordinary CI (compilers, type checks, tests): 1/20 and 0/20. Each run
  writes `results[-heldout][-baseline].json` and `.md`.
- `figures/make_figures.py` — the figures. Fig. 4 (two gate messages) is
  still drawn but no longer in the paper; it gave its space to the
  held-out results.
- `build_paper.py` — writes the text into the template. Every number in
  the evaluation, Table II and the abstract is read from the result files,
  and it asserts which held-out mistakes were missed, so the text cannot
  drift from the data. Citations are written `[@key]` and numbered by first
  appearance. The template is the Outreach `.doc` converted to `.docx` (not
  in this repository: it holds the earlier paper and its co-authors'
  details).
- Layout follows the IEEESTEC template: its own styles (`figure caption`,
  `table head`, Heading 5 for References), figure labels in Times New Roman
  at 8 pt or more, every figure and table at the top or bottom of a column.
  The preview matches Word's layout because the build gives the title
  section the body's margins and turns off automatic hyphenation (both
  artefacts of the .doc conversion).
- To export the PDF as committed, Times New Roman must be installed, then:
  `soffice --headless --convert-to 'pdf:writer_pdf_Export:{"UseLosslessCompression":{"type":"boolean","value":"true"},"ReduceImageResolution":{"type":"boolean","value":"false"}}' Fiducial_Zdravkovic_Aleksa.docx`

Rerun the tests, then the figures, then the paper, whenever the platform
changes what a gate catches. A rule added for a held-out miss makes that
set no longer held out: a later paper needs a new one.

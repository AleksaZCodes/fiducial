# A physical product is parts placed in regions of an outline, and the platform declares none of the three

**Date:** 2026-09-30
**Status:** decision superseded by `2026-09-30-hardware-is-a-capability.md` (built in the platform, not in the first product first); the context and the paper-readiness list stand
**Supersedes:** nothing

---

## Context

Two things were asked for together: get Fiducial ready for an IEEESTEC paper,
and use it to start building the reference product (a private repository) — PCB, BOM, a manufacturable
case whose outline is the product's logo. Doing the second is what exposed the gaps
in the first. Every gap below was checked against the repository on this date,
not inferred from the roadmap.

**1 · The `research` capability documents a pipeline that does not exist.**
`capabilities/research/SKILL.md` describes a `fid-research` executor that merges
`.bib` files, resolves DOIs and typesets through Pandoc. The capability has no
`pipelines/` directory, and `fid-research` appears nowhere in
`crates/fiducial-cli/src` — the executors that exist are `fid-adapters`,
`-advise`, `-brand`, `-deploy`, `-design`, `-identity`, `-legal`, `-mesh`,
`-schema`, `-thesis` and `-validate`. So the paper about the platform cannot
currently be written *with* the platform, which the roadmap's *Research &
authoring* item names as the point. (The same SKILL.md also misspells the
author's surname in its example.)

**2 · The EDA pipeline never runs an EDA tool.** `pipelines/eda.toml` runs
`fid-validate` over a hand-written `board.interface.json`. KiCad is not invoked
in CI, and the Gerber/drill pipeline is a comment. `SHIPPED.md` titles Phase 14
*"atopile → KiCad → `board.interface.json` + fab outputs"*; what shipped is the
schema and its validation. A paper must not claim the title.

**3 · The case is the board's bounding box, and it can only be a mesh.**
`fiducial-mesh` derives a sealed, fastened case — carefully, with ray-probed
through-holes — but from a rectangle: `SHIPPED.md` records that offsetting a
non-rectangular outline is "the one genuinely absent primitive". Its outputs
are STL and GLB. A manufacturer quoting injection moulding or CNC wants STEP,
and so does KiCad's 3D fit check. A mesh cannot become STEP.

**4 · Principle 5 has no implementation.** *"Every cost is therefore derived
and asserted, never estimated: bill of materials, material mass, machine time
…"* — there is no BOM, no cost field and no cost assertion anywhere in
`crates/`. The principle the mission cites Manu Prakash for is, today, prose.

**5 · There is no concept for a thing that is not a connector.**
`board.interface.json` knows connectors, and `CONNECTOR_OPENINGS` knows their
bodies, which is why the case can punch a USB-C opening. It has no way to say
*"a 60 × 60 mm solar panel sits in the lid"*, *"an 18650 holder makes the cavity
21 mm deep"* or *"the gas sensor needs a vent in the wall"*. Those are the facts
a sensor node is made of.

**6 · The paper has mechanisms but no measurements.** The brief
(`2026-09-14-disclosure-authorship-and-citation.md`) lists adversarial
verification and the Phase 18 negative result, both real. It has no number for
the claim itself: how many drift defects the gates caught, what a second
product cost compared with the first, how long `fid new` → deployed takes.

## Decision

**Four concepts, first built in the product that needs them** — the first product's
`hardware/`, commit `d200f1b` on `claude/elegant-ritchie-bxgij1` — and brought
into the platform when a second product needs them (MISSION anti-goal: *"a
capability … is generalized when a second one does"*).

| Concept | Is |
|---|---|
| **Outline** | any closed, straight-edged SVG shape, addressed by path and subpath index — the logo, unchanged, is the case |
| **Region** | another addressed shape in the same coordinates — an inner shape inside the logo's outline |
| **Part** | a body (size), a source (MPN, LCSC, second source), a price, a `status` (`decided`/`assumed` + what settles it) and a `place`: a region, `board`, or `wall` |
| **Feature** | what the case does for a part — `lid-pocket`, `wall-hole` |

**The scale is solved, not chosen.** For convex regions the fit of a
rectangle is exact — one half-plane per region edge on the rectangle's centre —
so the smallest scale at which every region-placed part fits is found by
bisection. The panel sizes the house, the house sizes the logo, the logo is the
case. The board is then the largest square within its declared ceiling
(100 mm, the cheapest fab tier) that fits the cavity; the cavity depth is the
tallest board part.

**One contract between solving and modelling.** `derive.py` (standard library
only) writes `layout.json`, `bom.csv` and a KiCad 7 board holding the outline
and the mounting holes. Those three are committed and gated by
`derive.py --check`. `cad.py` (build123d/OCCT) and `kicad-cli` read them and
produce STEP, STL, Gerbers and drill files as CI artifacts. The standoffs in the
case and the holes in the board are the same four numbers.

**Cost is asserted.** `[cost] node_ceiling` is declared first; an unpriced part
is listed and the verdict is "cannot be asserted yet", never an estimate.

First result, worth stating because it is the kind of fact the system exists to
surface: with a 60 × 60 mm panel the plate is **263 mm**. The plate is about
4.4× the panel's width because the house is about a quarter of the mark. That
is a brand decision and a power-budget decision interacting, and neither
document would have shown it alone.

## Why not the alternatives

- **Extend `fiducial-mesh` to polygons and STEP.** Polygon offsetting is
  feasible; STEP is a B-rep format and means writing a CAD kernel, which the
  mission names as an anti-goal. `fiducial-mesh` keeps what it is good at — a
  `no_std`, dependency-free GLB for the web — and a real kernel does the rest.
- **Build the capability in the platform now.** One product needs it. The
  interface (four concepts, `layout.json`) is the part worth getting right, and
  a second product is what tests it.
- **Start from atopile.** It generates schematic and netlist from code, which
  is the right long-term direction for the schematic, but it does not address
  the case, the regions or the cost. It can sit behind the `eda` contract later.
- **Hash the STEP/STL outputs.** OCCT tessellation is not byte-stable across
  versions; the gate would fail on a library upgrade and train people to
  re-derive without looking. The gate hashes the text the geometry comes from.

## Consequences

- A the reference product node now goes from `node.toml` to a manufacturing set in one command,
  in CI and in a Claude Code cloud session (KiCad 7 from Ubuntu 24.04 apt,
  build123d from PyPI).
- The paper gains a concrete cross-domain case — brand SVG → mechanical →
  EDA → BOM — that it can demonstrate rather than describe.
- Two runtimes now hold geometry: Rust (`fiducial-geometry`) and Python
  (`derive.py`). Generalizing means porting the half-plane solver into
  `fiducial-geometry`, where it is naturally `no_std`.

## What this deliberately does not do

- **Place or route the PCB.** The derived board is the frame. The RF path is
  laid out by a person.
- **Accept curves or non-convex regions.** Both fail loudly with the reason.
- **Estimate a price.** Ever.

---

## Paper readiness, ordered by cost of delay

| # | Item | Kind | Why here |
|---|---|---|---|
| 1 | Fix or retract the `research` SKILL.md — it describes an executor that does not exist | debt-accruing | Every agent that reads it will try to run it |
| 2 | Reword Phase 14's claim in `SHIPPED.md` to what shipped (schema + validation) | debt-accruing | The paper's evidence section is derived from this file |
| 3 | Tag a release and mint the Zenodo DOI | debt-accruing | The paper must cite a version; only the account owner can click it |
| 4 | Measure the claim: drift defects caught by each gate (from git history), time `fid new` → deployed, second-product cost (the first product vs Fiducial itself) | multiplying | Turns a systems description into an evaluation |
| 5 | Build `fid-research` minimally: `[paper]` + `references/*.bib` → Pandoc with the IEEE template | multiplying | The paper is then its own demonstration |
| 6 | Take the disclosure brief to an attorney | terminal but time-boxed | The brief says *before submission* |
| 7 | This spec's four concepts, generalized once a second product has hardware | terminal | Anti-goal: not before a second consumer |

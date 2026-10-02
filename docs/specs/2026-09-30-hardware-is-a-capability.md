# Hardware is a platform capability now, and pipelines track what they read

**Date:** 2026-09-30
**Status:** accepted
**Supersedes:** the *Decision* of `2026-09-30-a-physical-product-is-parts-in-regions.md` (build it in the first product first). Its *Context* stands.

---

## Context

The earlier record proposed building outline / region / part / feature in
The first product and generalizing when a second product needed it. The owner overrode
that the same day: the platform's gaps get closed **before** the node's case
is built, because every product after this one — the reference product's node, Ring of
Pursuit's hardware, whatever comes next — would otherwise start by
re-deriving the same machinery. That is principle 5b (spend upfront where it
compounds) outranking the second-use rule, and it is recorded here so the
next reader does not relitigate it.

Building the first version in the first product had already surfaced three facts the
design needs:

- **The outline is the logo, and logos are not convex.** The reference product's logo outline has a
  notch between its tips. The convex solver in the first version refused it.
- **"The panel fits in the house" was the wrong rule.** The house is a
  *window* the panel sits behind, so the house grows until it meets the panel
  (less its gasket overlap), not until the panel fits inside it. At the
  owner's declared panel the outline is 118.5 × 176.7 mm; under the old rule a
  60 mm panel made a 263 mm case.
- **The PCB was defaulted to its ceiling.** 100 × 100 mm is JLCPCB's cheapest
  tier, a limit, not a size. Sized from its parts, the reference product's board is 42 mm.

A fourth came from the owner while it was being built: the declaration reads
SVGs it does not own. When the logo is redrawn, every *output* still matches
its hash in `fiducial.lock`, so `fid derive --check` — which only hashed
outputs — passes, and the case silently disagrees with the logo.

## Decision

1. **`hardware` is a built-in capability** with an in-process executor,
   `fid-hardware`. The spatial reasoning (fit, scale, packing, screws, holes,
   heights, BOM, cost, assembly order) is Rust, in `fiducial-geometry::fit`
   (`no_std`, so the same solver can run in a browser configurator) and
   `crates/fiducial-cli/src/hardware.rs`. It writes text — `layout.json` with
   a `why` list — that a person, an agent and a CAD kernel all read, and that
   `fid derive --check` gates.
2. **Solids come from a real B-rep kernel behind that text**: build123d on
   OpenCascade, in a platform-owned `hardware/cad.py`. It makes no decisions;
   it offsets non-convex outlines, routes the gasket groove round the screw
   bosses, and **proves the product can be assembled** — no interference, every
   part insertable, the lid closes, screws engage — failing the build otherwise.
3. **Mounts are a vocabulary**, and the case features follow from them:
   `window`, `pocket`, `socket` (space reserved at the declared envelope even
   while the part is assumed), `smd`/`tht`, `bulkhead`. A new mount is a new
   row in `MOUNTS` and a branch in `cad.py`.
4. **Every build ends in a review pack** — assembled, exploded, section and
   inside renders plus a self-contained viewer — and the capability's SKILL.md
   tells agents to show it to a person after any geometry change. A physical
   design is not reviewed from numbers.
5. **Pipelines track inputs.** A pipeline may declare `inputs`; a built-in
   executor reports what it reads (`fid-hardware`: the declaration and every
   SVG it addresses). Their hashes go into `fiducial.lock [inputs]`, and
   `--check` fails naming the upstream file that moved. `layout.json →
   shapes_read` records each shape's vertex count, bounds and area, so the
   same redraw is visible in review as geometry.

## Why not the alternatives

- **All in Python.** Faster to write, and the kernel is Python anyway. But the
  gate would then need a Python runtime and a CAD kernel in every CI job, and
  the solver could not run in a browser or on a device. The line is drawn
  where determinism ends: the solve is exact text; tessellation is not.
- **OpenSCAD / CadQuery.** OpenSCAD is CSG-to-mesh and cannot emit STEP.
  CadQuery is the same kernel as build123d with a less composable API; the
  contract is `layout.json`, so swapping is a rewrite of one file.
- **Hash STEP/STL/Gerbers in the lock.** OCCT tessellation and KiCad's
  timestamps are not byte-stable across versions; the gate would fail on a
  tool upgrade and train people to re-derive without looking.
- **Regenerate-and-compare for input drift** (what `fid-schema` and
  `fid-adapters` do). It only works for executors that are pure functions of
  committed files and cheap to run; hashing inputs works for every executor,
  `shell` included, and names the file that moved rather than the output that
  would change.

## Consequences

- `fid add capability hardware` gives any product the whole loop. The reference product adopts it
  in `hardware/product.toml`, outline from a layer of its own brand SVG.
- `fiducial.lock` gains an `[inputs]` table, omitted when empty: products with
  no tracked inputs see no change.
- `requires_tools` names `python3`, `kicad-cli` and `node`; `fid doctor`
  reports them missing. The gate needs none of them.

## What this deliberately does not do

- **Place or route the board.** `board.kicad_pcb` is the frame; the positions
  in `illustrative_placement` exist for the renders only.
- **Accept curves.** A curve fails naming its command.
- **Estimate prices.**
- **Build `fid-research`.** The paper is not being written yet; the research
  capability's SKILL.md says plainly that it is not built.

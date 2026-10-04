# Hardware is solved by engines, not by a growing list of rules

**Date:** 2026-10-01
**Status:** accepted — P1–P6 shipped (P5 re-scoped, see *Progress*); P7 in *Progress*
**Supersedes:** the placement half of `2026-09-30-hardware-is-a-capability.md`

---

## Context

`fid-hardware` decides where every part goes, which way it turns and how its
wires run with hand-written code: a bounded search over the floor, a
ratsnest-scored search on the board, and one block of code per rule
(`near`, `away_from`, `zone`, `faces`, `edge_mm`, `pegs`, `follows`,
`through_wall`, overhangs, the wire router). `hardware.rs` is about 5,000
lines and grew one product request at a time — every the reference product review round added
rules. Each new rule has to be taught to interact with every old one by hand;
the number of interactions grows with the square of the rules. That is the
explosion the product owner named: "we're adding a lot of rules for edge
cases, and that can be infinite."

Evidence that it already bites:

- placement failures that were rule interactions, not geometry: the radio module
  could not fit because the snap hook, the edge strip and a peg each took a
  corner the others assumed free (fixed by hand, three times);
- grid-snapping misses in the hand search (a zone sized exactly for a part
  that the stepping could not hit);
- the routing failures of 2026-10-01 were a tool interaction (a windowed
  keep-out crashing Freerouting, hole keep-outs padded twice), found only by
  reading Java stack traces.

## Decision

Fiducial owns the **declaration, the derivation contract, the gates and the
agent context**. Proven engines own the **solving and the verifying**. A rule
in `product.toml` becomes *data* — a constraint or a cost term handed to an
engine — never a new code path.

| Concern | Engine | Contract fiducial owns |
|---|---|---|
| Where parts go, which way they turn (case floor and board) | **OR-Tools CP-SAT** (constraint programming; `NoOverlap2D`, boolean rotations, linear wirelength) | `generated/placement-model.json` (the problem) → `generated/placement.json` (the answer), both gated |
| Solid geometry, fit, insertion, seal | **OpenCascade** (build123d) | `layout.json` → `cad.py` checks — unchanged |
| Board rules, routing, fab output | **KiCad** DRC/ERC, **Freerouting** | `board.kicad_pcb` → `route.py` — unchanged; hand routing added |
| Circuit correctness | **ngspice** | a check that the power circuit's set points (VOUT, VBAT_OV, VBAT_OK) are what `product.toml` claims |
| Wire paths | A* on a grid — a textbook algorithm, kept in `wires.rs` | `layout.json → wires[].cores_mm` |

The solver is a **tool behind a contract** (MISSION principle 6): the model
file states the problem in fiducial's terms; `hardware/place.py` turns it into
a CP-SAT model and writes the answer. Another solver that reads the same model
is a one-file change. CP-SAT runs single-threaded with a fixed seed, so the
answer is deterministic and can be gated by hash like every other artifact.

### The placement model (the contract)

Every placeable thing is an **item**: a size per allowed rotation, an allowed
region (a zone, the board, the cavity), optionally a fixed position, and its
pads (offsets per rotation, each on a net). Constraints and costs are
**generic kinds**, not per-rule code:

| Kind | Hard / cost | Covers today's rules |
|---|---|---|
| `inside` (item in a rectangle set) | hard | `zone`, `board`, `edge_mm`, cavity |
| `no_overlap` (with clearance) | hard | parts, keep-outs, hooks, pegs, bosses |
| `fixed` (position, rotation) | hard | `at_mm`, `rotate_deg`, `follows` |
| `facing` (a side toward an edge) | hard | `faces`, `faces_pin`, `through_wall` |
| `apart` (min distance between items) | hard | `away_from`/`away_mm` |
| `near` (distance to a point or item) | cost | `near`, `board.near` |
| `net` (half-perimeter wirelength of a net's pads) | cost | ratsnest placement, decoupling beside its pin |
| `toward` (a pad row faces a direction) | cost | pads square to their wire |

A new requirement is a new **instance** of a kind in a declaration. A new
**kind** is the rare platform change, and it is one constraint builder, not a
search rewrite.

## Why not the alternatives

- **Keep the hand search, add rules carefully.** Every rule is a code path
  that must be taught every other rule; the cost of the next rule rises with
  the count. This is the status quo, and the evidence above is its bill.
- **CP-SAT linked into the Rust binary.** OR-Tools is a large C++ build; the
  Rust bindings need it installed. Every contributor and every CI job would
  build or vendor it. The Python wheel installs in seconds and `build.sh`
  already runs Python. The contract makes the language irrelevant.
- **A MILP solver (HiGHS) or an SMT solver (Z3).** Both can express packing;
  CP-SAT is the strongest open solver on exactly this shape of problem
  (disjunctive non-overlap, booleans, bounded integers) and is what the
  rectangle-packing literature benchmarks against.
- **A geometric constraint solver (SolveSpace, PlaneGCS) for placement.**
  These solve *relations* (flush, tangent, aligned), not *search* (where
  among many options). Kept in reserve for the relations `cad.py` hand-codes
  today (a flush pocket, a roof on a panel's edge); not needed yet.
- **A commercial AI placer/router (Quilter, DeepPCB).** Closed, paid,
  per-board; cannot be gated in CI. Hand routing (below) is the escape for
  boards beyond the autorouter.

## Consequences

- `fid derive --pipeline hardware` runs `hardware/place.py`, so the hardware
  derive gate needs Python and `ortools` (CI installs them; `build.sh` already
  needed Python).
- Placement quality becomes *optimal for the declared costs* rather than
  "first good spot", and a failure becomes **infeasible, with the conflicting
  constraints named** (CP-SAT's assumptions core), not a guess at which rule
  to relax.
- The hand search, the ratsnest search and most per-rule placement code are
  deleted. Rule interactions stop being fiducial's code.
- Weights between cost terms are declared (`[placement.weights]`), with
  defaults; tuning them is a declaration change, gated like any other.

## What this deliberately does not do

- It does not design a high-speed board. A Raspberry Pi-class board is routed
  by a person (or a careful agent) in KiCad; fiducial gates the result
  (`board.routing = "hand"`).
- It does not replace the solid kernel's checks: the solver places
  envelopes; OpenCascade still proves the real shapes fit.
- It does not route wires with CP-SAT: a wire path is a shortest-path
  problem, solved exactly by A*.

---

## Plan

Order by cost of delay (MISSION 5c): what accrues debt first.

| Phase | What | Done when |
|---|---|---|
| P1 | Placement contract + CP-SAT backend for **board** parts | the reference product solves through `place.py`; every board rule above expressed as a kind; tests for each kind; the ratsnest search deleted |
| P2 | Same for the **case floor** (sockets + board) | the bounded floor search deleted; the reference product and the seed solve |
| P3 | Infeasibility explained: the named minimal set of conflicting constraints | a test with two contradictory rules fails naming both |
| P4 | Hand routing: `board.routing = "hand"` — the product keeps `hardware/board-routed.kicad_pcb`; the build checks footprints, positions and nets match the declaration, DRC passes, autorouter skipped | a test that a moved footprint fails |
| P5 | ~~ngspice~~ a DC check of declared set points, solved from the netlist (re-scoped 2026-10-04) | a wrong resistor fails derive by name; the example's USB-C CC windows checked |
| P6 | The minimal showcase product (USB-C air-sensor stick: RP2040, SHT40, WS2812, USB-C through the wall; firmware + web over one protocol) | builds end to end in CI |
| P7 | Paper evidence: rules → constraint instances, lines deleted, the reference product before/after | numbers in this file |

## Ledger — every product-owner request in this thread, and its state

Kept here so it survives a context reset. Update it with every commit.

| # | Request | State |
|---|---|---|
| 1 | Escape hatch: the product adds its own geometry | done — `hardware/shapes.py`, `model =` (eb11b33) |
| 2 | Hand routing, gated | done (P4): `board.routing = "hand"` |
| 3 | Component orientation always sensible, esp. along a wire | wires/pads done (eb11b33); every free part's turn chosen by the solver to shorten its connections (P1 done) |
| 4 | Explicit user constraints always win | done (P1): `at_mm`, `rotate_deg`, `faces`, `zone`, `away_from` are hard; a conflict fails naming them (test `a_declared_position_and_turn_are_kept…`) |
| 5 | Wire engine: avoid collisions, dressed like real wires | done — `wires.rs` (eb11b33) |
| 6 | Antenna fed from its near end | done (eb11b33) |
| 7 | How coherent fiducial is, for the paper | answered 2026-10-01; strengthened by P1–P3, P7 |
| 8 | Minimal cheap showcase example | **P6** |
| 9 | Existing engines vs building it ourselves | this spec |
| 10 | Track every request; don't forget | this ledger |
| 11 | No Raspberry Pi demo | dropped |
| 12 | the reference product layout notes (gas sensor central, power pads turned, programming pads at an edge, perimeter margin) | done (e701601, eb11b33) |
| 13 | Datasheets fetched + AI-ready parts context | done — `parts.py`, `PARTS.md` (e701601); vendor PDFs blocked by this environment's network |
| 14 | PRs open | AleksaZCodes/fiducial#90, and the product's own |
| 15 | Sensor isolated: room round the gas sensor for a small sealed tube from the vent, so outside air and water reach the sensor, not the cavity | done — vent `seals_to`: a chimney printed with the lid on a squeezed silicone ring, leaning up to 3 mm. In the model: a keep-out, a region (ring on the board) and one new kind, `within`. The reference product: membrane 10 → 8 mm and the radio's left band became `near = "left"` — both named by the solver as the conflict; **open question to the owner** which should give |
| 16 | Isolation (not waterproofing): a smaller ring, wholly on the board, landing on a uniform surface; radio on the left; board may get narrower later | done — `membrane = "outside"` (10.2 mm ring, was 13.8), F.Cu track/via keep-out under the ring, hook lip counted in side bands so the radio's left band fits again. Narrower board: after P2 |
| 17 | Look back: fix what was slow or buggy, note the rest; paper next | done — see *Retrospective* |

## Progress

- 2026-10-01 — spec written; ortools 9.15 verified installable; starting P1.
- 2026-10-01 — **P1 done.** Board placement goes through
  `placement-model.json` → `hardware/place.py` → `placement.json`; the
  ratsnest search, the placement-order chain (`depth_of`) and the per-rule
  targets are deleted. Kinds shipped: inside, no-overlap, fixed, flush,
  apart, near, net, plus a small turn tie-break. Findings worth keeping:
  - **CP-SAT with several workers is not deterministic**, not even with
    `interleave_search` and deterministic limits (two runs, two answers).
    One worker is, but stalls on its first solution. `place.py` runs its own
    large-neighbourhood search instead: one worker, alternating passes that
    free a part with its nearest neighbours, then a part with every part it
    shares a net with (a circuit moves together). Deterministic, verified.
  - Exact-fit zones need bounds rounded to the nearest grid step from the
    board's corner, not floored/ceiled from absolute coordinates.
  - The answer is cached by model hash, so `--check` needs no Python; only a
    changed model runs the solver (~1 min for the reference product at effort 10).
  - **P3 came early:** infeasible models report a minimal set of
    conflicting declarations (core from assumptions, then deletion-minimised);
    holes and hooks are named obstacles.
  - Kinds now: inside, no-overlap, fixed, flush, within, apart, near, net.
  - Lesson (the reference product, chimney round): `near` is a cost, so under the solver it
    yields to connection length — the reference product's SWD pads drifted 16 mm off the edge
    the owner asked for. A requirement must be declared as a hard kind (a
    zone band, `at_mm`, `faces`); `near` is for preferences only. The skill
    says so; the old search hid this by placing `near` parts first.
  - the reference product: wirelength objective 9,087 (old search) → 7,908 (−13%); routed first
    or second attempt, DRC 0, all case checks pass. Lines: `hardware.rs`
    +389/−354 (the model builder is as long as the search it replaced — the
    gain is that a new rule is a new instance, not a new interaction), plus
    `place.rs` 155 and `place.py` 328.

- 2026-10-04 — **P4 done.** `board.routing = "hand"`: the product keeps
  `hardware/board-routed.kicad_pcb`, routed in KiCad from the derived board.
  `fid derive` reads it (an input like any other) and fails while a part has
  moved, turned, flipped, been added or removed, a pad is on another net, or
  the outline differs — each named. The build routes nothing: the declared
  rules and KiCad's DRC decide it. Tested both ways, on a board KiCad itself
  saved (KiCad 7's `fp_text reference` and KiCad 8's `property`), with
  routing left to the person: tracks and zones are not compared.

- 2026-10-04 — **P2 done.** The case floor (sockets, either way round, and
  the board) is a second model, `floor-model.json` → `floor.json`, solved by
  the same `place.py`. The cavity can be any outline, so fid computes, per
  item and turn, the rectangles where its centre fits (half-millimetre rows,
  a tenth where they change, ends bisected to a hundredth) and the solver
  chooses among them; two kinds were added for it, generic like the rest:
  `inside` per turn as a set of centre rectangles, and `near` with slack per
  axis (cover a point rather than centre on it). Screw bosses are obstacles
  that need not clear each other. The bounded search and its anchors are
  deleted; `case.grid_mm` no longer touches the floor. The answer is checked
  against the exact fit. On the sensor stick the board now stands at its
  exact limit (0.69 mm mouth, no workaround); a part that fits nowhere, and
  two that fit apart but not together, fail naming them.

- 2026-10-04 — **P5 done, re-scoped.** The power circuit's set points were
  the reference product's regulator dividers; the public example has none,
  but its USB-C pull-downs are a set point of the same kind (the CC voltage
  a source reads must land in the Type-C spec's window for each source
  current). So the check is generic: `[[check]]` drives nets and expects
  others in a window, solved from the declared resistors by nodal analysis
  on every derive, failing by name. A resistive DC network is a textbook
  linear solve, kept in Rust as A* is for wires (`circuit.rs`); ngspice is
  not needed for it, and it stays the engine for anything nonlinear or
  transient, which no check can yet ask for. The example declares three
  checks (default, 1.5 A and 3 A sources); a 22 kΩ in place of 5.1 kΩ fails.

## Retrospective — 2026-10-01, before the paper

What was slow, buggy or missing in today's work, and what was done about it.

Fixed:
- **Capability caches were embedded.** `build.rs` embedded every file under
  `capabilities/`, so a `__pycache__` left by running a capability's Python
  broke the build (twice). It now skips interpreter caches.
- **A band on a snap board's side ignored the hook's lip.** Zones were sized
  exactly for their part, so a hook made a part fit only above or below it.
  Bands on the hooked sides are deeper by the lip (`HOOK_LIP`).
- **Placement jumped on every re-solve.** The previous answer now seeds the
  search (outside the model's hash): solves converge instead of wandering —
  The reference product's objective fell another 10% and a re-solve reproduces it.
- **The review renders reloaded every mesh per view.** `render.mjs` loads
  the scene once and moves the camera: 115 s → 85 s.

Measured (the reference product, one full `build.sh`, cached placement): routing 40 s (two
Freerouting attempts), `cad.py` solids and checks 27 s, KiCad exports 4 s,
renders 85 s; a changed placement model adds ~65 s of solving.

Known, not fixed (in order of what bites first):
1. **Renders are software WebGL** (~10 s a view). A GPU, or fewer shadow
   passes in `viewer.html`, would cut most of it; `SKIP_RENDER=1` skips them.
2. ~~Floor placement is still the old bounded search~~ — done 2026-10-04 (P2).
3. **Freerouting is not deterministic**: the routed board differs run to
   run; the build retries until 0 open. Hand routing (P4) is the escape.
4. **No "stay put" cost**: a re-solve may still move parts when it finds a
   shorter layout. A cost toward the previous answer would trade length for
   stability, if wanted.
5. ~~Wires are checked against the chimney only~~ — fixed 2026-10-02:
   every core against every solid (`making-the-thesis-true.md`).
6. ~~The dev `fid` reports `adapters.generated.ts` stale~~ — explained
   2026-10-02: the generator changed, not the inputs; the message now says so.
7. **The sensor's decoupling cap sits outside the chimney ring**, about 3 mm
   from its pin through a via pair; fine at 100 nF, worth a look in review.

## Resuming after a context reset

Branch `claude/elegant-ritchie-bxgij1`. Read this file, then
`crates/fiducial-cli/src/hardware.rs` (board placement: search for
"Ratsnest placement"; floor placement: "Search: each item tries"), then
`capabilities/hardware/hardware/` (`cad.py`, `route.py`, `parts.py`).
The reference product (a private repository) is the product every change is
verified on: `PATH=<fiducial>/target/release:$PATH bash hardware/build.sh`,
then look at `hardware/build/review/*.png`. Its CI pins a fiducial commit in
`.github/workflows/ci.yml`; bump it with every fiducial push the reference product depends on.

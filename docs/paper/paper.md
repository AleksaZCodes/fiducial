# Fiducial: Build any robust multidisciplinary product with AI agents

**Aleksa Zdravković** (ORCID 0009-0004-1535-1839)

*Draft — 2026-10-02. Cites Fiducial `v0.9.x`; the version DOI is added when
the release is minted. Every claim below points at a file, a test or a CI
job in the repository at that release.*

---

## Abstract

A physical product is many disciplines at once: a case, a board, firmware, a
web page, a bill of materials. The same facts (a pin, a part's size, a net's
name) are written into each, the copies drift, and the drift is found late.
AI agents make this worse and better at once. They write each copy faster,
and each one sees only its own corner. Fiducial is a platform in which every
fact is **declared once**, every artifact is **derived** from the
declaration, and a **gate** fails the build when any artifact no longer
follows from it. Hard problems are delegated to proven engines behind fixed
contracts: CP-SAT places parts, OpenCascade proves the solids fit, KiCad and
Freerouting route and check the board. We show, on a public example product
built in the platform's own CI, that:

- the output is **buildable**: real, verified, orderable parts and fab files;
- the method is **general**: every placement rule is an instance of eight
  generic constraint kinds, not product-specific code;
- it is **robust**: drift fails CI, and contradictions are named;
- it is **multidisciplinary**: one pin change reaches the board, the
  schematic, the firmware and the web types in one derive, and stale code
  stops compiling;
- it works **with AI agents**: one person and one agent built the example in
  an afternoon, and the record shows which of 23 refusals were the
  platform's (16) and which no gate caught (3).

The judgment that decides what a product is, and the testing on a bench,
stay human. Fiducial is the deterministic layer that bounds the rest.

---

## 1 · Introduction

A small product, say a USB stick that reports a room's temperature, carries
the same fact in many places. The sensor's data pin is in the schematic, the
board's copper, the firmware's pin setup, the test fixture and the
documentation. The socket's size is in the footprint, the case's opening and
the render. Each copy is typed by someone, or something, that knew the fact
at the time. When the fact changes, every copy must change with it, by
memory. Drift between them is found when a board comes back from the fab,
or when a case does not close.

Teams pay for this with process: reviews, checklists, a person who owns the
pinout. A single person has no such process, and an AI agent that writes the
firmware has no idea a board exists. Agents are fast within a codebase and
blind across them. Scattered codebases, each with an agent that knows only
its own corner, give incoherent products.

Fiducial takes a different route. Its thesis (`MISSION.md`) is **declare
each fact once; derive every artifact from it.** A product is one repository
with typed declarations. Artifacts are derived from them, never hand-edited,
and recorded in a lock. A gate, `fid derive --check`, runs on every commit and
fails when any artifact no longer follows from its declaration. Agents work
inside those gates. They change declarations, run derivations, and are
refused, by name, when they get something wrong.

This paper makes one claim, narrower than the title and evidenced:

> A single person working with AI agents can carry a cross-domain product to
> a manufacturable, checked state when every fact is declared once, every
> artifact is derived and gated, and hard problems are delegated to proven
> engines behind fixed contracts — so the system grows by declarations, not
> by rules.

Section 2 describes the method and Section 3 the implementation. Section 4
presents the worked example, and Section 5 the evaluation. Section 6 is a
discussion of where human judgment stays, Section 7 the limits, and Section 8
related work.

## 2 · Method

### 2.1 Declare, derive, gate

A **declaration** is a typed file a person writes: `fiducial.toml` for the
product, `hardware/product.toml` for its physical facts, an SVG for its
outline. A physical fact carries its unit; a part carries its provenance
(where its size came from) and its status (`assumed` until chosen).

A **derivation** is a pure function from declarations to artifacts, run by
`fid derive`. Its outputs are recorded in `fiducial.lock` with their hashes
and the hashes of the inputs they came from.

A **gate** is a check that fails the build. `fid derive --check` fails when:
- an artifact's hash differs from the lock (it was hand-edited);
- an input changed since its pipeline last ran;
- regenerating it gives a different file.

The third case catches the opposite of a hand-edit, a declaration that moved
without a re-derive. The product's CI runs the gate on every push.

### 2.2 Engines behind contracts

The first version of the hardware pipeline placed parts with hand-written
search. It had one block of code per rule (`near`, `away_from`, `zone`,
`faces`, …). Each new rule had to be taught every old one, and the
interactions grew with the square of the rules. Three times, a part did not
fit because three rules each assumed a corner the others had taken.

The decision (`docs/specs/2026-10-01-hardware-is-solved-by-engines-not-rules.md`)
was that **Fiducial owns the declaration, the contract, the gates and the
agent context; proven engines own the solving and the verifying**:

| Concern | Engine | Contract |
|---|---|---|
| Where parts go, which way they turn | OR-Tools CP-SAT | `placement-model.json` → `placement.json`, both gated |
| Solid geometry, fit, insertion, seal | OpenCascade (build123d) | `layout.json` → `cad.py` checks |
| Board rules, routing, fab output | KiCad DRC, Freerouting | `board.kicad_pcb` → `route.py` |
| Wire paths inside the case | A* on a grid | `layout.json → wires[].cores_mm` |

A rule in a declaration becomes **data**: an instance of a constraint kind
handed to the engine. Eight kinds cover every placement rule the platform
has met:

| Kind | Hard or cost | Covers |
|---|---|---|
| `inside` | hard | zones, the board, edge bands, the cavity |
| `no_overlap` | hard | parts, keep-outs, hooks, pegs, bosses |
| `fixed` | hard | declared positions and turns |
| `flush` / `facing` | hard | a side to an edge, a socket through a wall |
| `within` | hard | a part inside a ring (a sensor under a sealed chimney) |
| `apart` | hard | minimum distances |
| `near` | cost | preferences for position |
| `net` | cost | connection length (half-perimeter wirelength) |

A new requirement is a new instance; a new kind is a rare platform change,
one constraint builder. An infeasible model is reported as a **minimal set of
conflicting declarations**, from CP-SAT's assumption core and then minimised
by deletion, so a contradiction is named rather than guessed at.

### 2.3 Parts as packages

A part may be pinned by its LCSC number, or declared by a requirement
(`kind`, `value`, `package`, `tier`). `parts.py resolve` answers each
requirement from an open catalogue of the JLCPCB/LCSC range, verifies every
pinned number against LCSC's own record, and writes `hardware/parts.lock`,
as Cargo resolves a version range to a locked version. Derive reads the lock,
and fails on an unanswered or changed requirement. Parts KiCad lacks are
vendored through easyeda2kicad, converted to the current format, and hashed.
The lock is committed, so CI never needs the network.

### 2.4 Agents inside the gates

Every product carries its agent context: `AGENTS.md`, and one skill file per
capability describing its declarations, what is derived, and what is
refused. These skill files are platform-owned and refreshed on every
`fid upgrade`. A guard hook runs before an agent's shell commands. The agent
writes declarations and product code, runs derive and the build, and reads
the refusals. When a refusal is a platform bug, the fix goes into the
platform, with a test, so every later product gets it.

## 3 · Implementation

Fiducial is a Rust workspace (fifteen crates; about 940 tests) and a
command-line tool, `fid`. A product is scaffolded by `fid new` and extended
by capabilities (`fid add capability hardware`, `firmware-rp2040`, web
frameworks, and others), each bringing templates, declarations, pipelines
and a skill file. `fid upgrade` merges platform changes into a product
three-way, against the base recorded in the lock, and refuses a merge that
would drop a declared section.

The hardware capability is the largest. `fid derive --pipeline hardware`
(executor `fid-hardware`, in Rust) solves the declaration:
- it sizes the case from the outline and the parts;
- it builds the placement model and calls `place.py` (CP-SAT, one worker, a
  deterministic large-neighbourhood search) for the board;
- it writes the layout, the KiCad board with every pad on its net, the
  schematic, the BOM, the assembly order, and the interface: `interface.json`,
  TypeScript types, and an Embassy pin map for the firmware.

`hardware/build.sh` then turns that into what a manufacturer and a reviewer
need. It routes and pours the board and runs KiCad's DRC (`route.py`). It
builds the solids and checks fit, insertion, seal and interference
(`cad.py`). It writes Gerbers, the assembly BOM and the placement file
(`fab.py`), and renders a review pack. The Python is only where an engine's
maintained interface is Python: build123d, OR-Tools, KiCad's `pcbnew`.

## 4 · Worked example: the sensor stick

`examples/sensor-stick/` is a USB-C stick that reports temperature and
humidity to a web page. It is small and cheap on purpose: an RP2040, an
AHT20 sensor, a WS2812B LED, a USB-C socket, 21 parts, about $5.14 of parts
per board at five boards. It still carries every discipline: a printed case,
a routed board, firmware and a web page.

A person writes five things:
- `hardware/product.toml`: the outline, the case, every part with its LCSC
  number, KiCad symbol and footprint, and what each pin connects to;
- `hardware/outline.svg`: the shape;
- vendored parts from LCSC's library, with their provenance;
- the firmware (`firmware/`): read the sensor once a second, send the reading
  as a `fiducial-protocol` frame over USB serial;
- the page (`web/src/`): connect over Web Serial and show the reading.

Everything else is derived and gated:
- the layout and why each thing is the size it is;
- the solved placement;
- the board and the schematic;
- the BOM and the assembly order;
- the TypeScript interface;
- the firmware's pin map.

`build.sh` produces:
- the routed board, DRC-clean;
- the case as STEP and STL, 36 × 84 × 15.5 mm, with the socket flush with
  its face and the sensor alone at the far end under a chimney to the
  vent;
- every fit and assembly check passing;
- fab files with no unsourced part.

The example's CI job checks every claim in its README on every push:
- derive is fresh;
- the board routes with no open connection and no DRC error;
- every solid check passes;
- every part is orderable;
- the firmware compiles and its host tests pass;
- the page's test passes and it type-checks.

It is **designed and checked, not built**: nobody has ordered, soldered or
plugged one in.

## 5 · Evaluation

### E1 · Buildable

Every part in the example is a real part, verified against LCSC on the day
it was resolved. Two candidate numbers were refused: one does not exist, one
had no stock. `fab/README.md` lists zero unsourced parts. The board passes
KiCad's DRC against the declared process (0.15 mm tracks and 0.127 mm
clearance, JLCPCB's standard two-layer rules), and the solids pass every
check `cad.py` makes: interference, insertion order, seal, lid fit.

### E2 · Multidisciplinary: one change, every discipline

The test
`a_moved_pin_reaches_the_firmware_and_a_renamed_net_breaks_code_still_using_it`
is the demonstration:
- swapping two pins in `product.toml` fails `--check` until derived;
- after the derive, the board's copper, the schematic and the firmware's pin
  map have moved, and a firmware stub compiles **unedited** against the new
  pins;
- renaming a net changes the schematic and the TypeScript type, and the stub
  still using the old name fails `rustc`, naming it.

On the example, moving the sensor's data line to a pin that is not an I²C
data pin makes the firmware fail to compile, as it should: Embassy's types
encode which pins can carry which signal. Nothing is copied by hand, so
nothing can be left behind.

### E3 · General: rules as data

Board placement moved from hand-written search to CP-SAT. On the private
reference product, the wirelength objective went from 9,087 (hand search) to
7,908 (−13 %), and seeding each solve with the previous answer cut it by a
further 10 %. The placement code is about as long as the search it
replaced, but the gain is in its shape. A new rule is a new instance of a
kind, not a new interaction with every rule before it.

Two contradictory declarations fail naming both
(`a_declared_position_and_turn_are_kept_and_two_that_collide_are_named`). On the
reference product, when two of its declarations could not both hold, the
solver named the two; the person chose which should give.

### E4 · Robust: what the gates refused

The example was built once by one person and one agent in an afternoon, and
rebuilt from scratch on the fixed platform for its guide
(`docs/guides/sensor-stick.md`). The first build's refusals are kept as a
case study: 23 of them, each at the step that made the mistake.

| Refused at | Count |
|---|---|
| parts (resolve, vendoring) | 3 |
| derive | 5 |
| route / DRC | 5 |
| solid checks | 2 |
| doctor / upgrade | 3 |
| firmware, render | 2 |
| review only (no gate) | 3 |

**Sixteen of the 23 were the platform's**, fixed in the platform with a test:
- a seal described for a case without one;
- a derived board always square;
- footprints whose courtyards missed their own pads;
- a merge base that never moved;
- one product's colour as every product's default.

The other seven were the product's: a part with no stock, a library land
pattern that failed the board's own rules.

**Three no gate caught, all at the USB-C socket.** Its mouth faced into the
case, it stood back from the case's face, and the lid left a slot open
above it. A person found them by looking at the renders. One then became
a gate: a through-wall socket must be flush within 1 mm, in a window of its
face plus 1 mm. The slot was fixed in the platform (the lid now fills it),
but nothing checks for a slot. The facing remains a judgment a person, or
an agent asked to look, has to make.

### E5 · Determinism, and where it ends

Placement is deterministic: CP-SAT with several workers was not (two runs,
two answers), so `place.py` runs one worker with its own neighbourhood
search, and the answer is cached by the model's hash. Routing is not
deterministic. Routing the example's one placement five times gave five
different boards (337 to 395 tracks, 23 to 28 vias), all DRC-clean. One
earlier route in four had failed DRC by 1.8 µm, because Freerouting routes
at the clearance exactly. The router now gets 10 µm more than the rule KiCad
checks. `route.json` records the placed board's hash and the routed copper's
side by side, so a re-route that differs is visible rather than silent.

### E6 · The platform improves by use

Building the example found platform bugs that unit tests had not, each
fixed where every later product gets it:
- an upgrade that reported a clean merge while emptying the product's list
  of capabilities;
- wires drawn but never checked, which, once checked, were found running
  through the lid;
- a socket's mouth read from a courtyard margin rather than its drawn body.

The plan that made the title true
(`docs/specs/2026-10-02-making-the-thesis-true.md`) records each, and what
is still not done.

## 6 · Discussion: judgment stays human; Fiducial bounds the randomness

**Better models move the line, not the need.** Even with the best models, a
person still questions the output, applies judgment and tests it. As models
improve, more is handed over and the checking gets lighter. But a
probabilistic system cannot be guaranteed correct, nor guaranteed to match
a product's constraints and wants. Even a deterministic tool is only right
about what it was told.

**What Fiducial is for.** It is the layer that reduces that randomness: a
deterministic way to build every part of a product in one place, with one
shared context. Declarations are the constraints, derivations are
repeatable, and gates refuse drift. An agent's mistake becomes a refusal at
the step that made it, by name. It no longer surfaces as a board that comes
back wrong.

**Product-oriented, not codebase-oriented.** One declaration read by every
agent, across hardware, firmware and web, makes a product designed as a
whole.

**What the gates could not judge** in this work, and a person did:
- a parts catalogue that changed format overnight (one basic part in
  53,012 on the day it was first read);
- a substring match that took "±0.1 %" for "1 %";
- whether a sensor should be isolated from the board's heat;
- which way a socket faces.

The record (E4) puts a number on it: 3 of 23 mistakes in one build were
caught only by looking.

## 7 · Limits

- **N is small.** One public example and one private product. "Any" in the
  title is a claim about the method (generic kinds, engines behind
  contracts), tested on two products. It is not a measurement over many.
- **Nothing is manufactured yet.** "Orderable and checked", not built.
- **Routing is not deterministic.** It is made visible (E5) and kept
  DRC-clean, not made repeatable; hand routing, gated, is the planned
  escape.
- **Floor placement is still a bounded search** with a step
  (`case.grid_mm`); moving it onto the solver is planned.
- **The board is always a rectangle.** A board cut to follow its case is the
  next step; the example made its case's chamfer smaller instead.
- **Power circuits are computed, not simulated.**
- **Some judgment has no gate**, by nature (Section 6).

## 8 · Related work

- **Code-defined hardware** tools describe circuits as code: atopile, SKiDL.
  Fiducial uses KiCad's own libraries and formats as the target rather than
  a new language, and adds the case, the firmware and the web from the
  same declaration.
- **Code CAD** describes solids as code: OpenSCAD, CadQuery, build123d. The
  platform uses build123d as its engine; the solids are derived, not
  authored.
- **Lockfiles and reproducible builds**: Cargo, npm, Nix, Bazel. Fiducial
  borrows the lock (requirement → exact part, hashed) and the idea that an
  output must follow from its inputs.
- **Infrastructure as code** popularised the plan/drift check
  (`terraform plan`). `fid derive --check` is that check, for a product's
  artifacts.
- **AI placement and routing services** (Quilter, DeepPCB) are closed and
  per board, and cannot be gated in CI. The platform keeps an open autorouter
  behind a contract and a gated hand-routing escape.
- **Coding agents** work within one repository and its tests. Fiducial gives
  them one product-wide repository whose tests include every discipline.

## 9 · Conclusion

Declaring each fact once, deriving every artifact and gating the result
turns a multidisciplinary product into one checked whole that one person and
an agent can carry to an orderable state. Delegating search and verification
to proven engines, behind contracts Fiducial owns, keeps the platform
growing by declarations rather than by rules. The record of building the
example shows where this stops: a person still decides what the product is,
still judges what no gate can, and still has to build one on a bench.

## Availability

Source, the example and every specification cited:
`github.com/AleksaZCodes/fiducial`, release `v0.9.x` (DOI to be added on
release). The example builds in the repository's CI (`example (sensor-stick,
end to end)`).

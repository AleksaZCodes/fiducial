# fiducial:hardware — a physical product, declared once

This product has the `hardware` capability. Its case, board frame, bill of
materials and assembly order are **derived** from one file,
`hardware/product.toml`. Nobody draws the case and nobody picks its size.

## The loop

```sh
fid derive --pipeline hardware     # solve → hardware/generated/  (gated by --check)
python3 hardware/parts.py resolve  # answer every `pick` from the JLCPCB catalogue → hardware/parts.lock
python3 hardware/parts.py sync     # vendor KiCad symbols, footprints, models → hardware/lib/
hardware/build.sh                  # route, solids, checks, fab files, review pack → hardware/build/
```

| Output | Is | Gated |
|---|---|---|
| `generated/layout.json` | every solved position, size, hole — and a `why` list saying what set each | yes |
| `generated/bom.csv` | declared parts, plus what the design implies: screws, gaskets, printed parts | yes |
| `generated/assembly.md` | the order it goes together in, from each part's mount | yes |
| `generated/board.kicad_pcb` | outline, mounting holes, every board part's real footprint placed and turned, every pad on its net, keep-out zones | yes |
| `generated/board.kicad_sch` | the schematic: each part's KiCad symbol, a net label on every connected pin, a no-connect on every other — one row per circuit | yes |
| `generated/interface.json`, `interface.ts` | what firmware and web read from the hardware: board size, sockets and their windows, every net and the pins on it — `interface.ts` makes the nets a TypeScript type | yes |
| `generated/board.rs` | with `[firmware] mcu = "<part id>"`: per net on an MCU I/O pin, a macro taking that pin from Embassy's peripherals (`board::sensor_sda!(p)` → `p.PIN_4`) | yes |
| `build/board-routed.kicad_pcb` | the board to fabricate: routed by Freerouting (or the product's own hand-routed board with `board.routing = "hand"`), ground-poured both sides, DRC'd by KiCad (`route.json`, `drc.rpt`) — fails the build if anything is unconnected or an error. Freerouting is not deterministic: `route.json` records the placed board's hash and the routed copper's, so a re-route that differs is visible | built |
| `build/*.step`, `*.stl`, `assembly.step` | every solid, for a manufacturer and a slicer | built |
| `build/checks.json` | interference (keep-outs included), insertion, lid, screw engagement, seal squeeze | built, fails the build |
| `build/fab/`, `schematic.pdf` | `gerbers.zip` (Gerbers and drill of the routed board), `bom-assembly.csv` and `cpl.csv` (an assembly service's BOM and placement), and `README.md` listing every placed part without an `lcsc` number — what cannot be ordered assembled yet; the schematic | built |
| `build/review/*.png`, `viewer.html` | assembled, exploded, section, inside, underside, board; an interactive viewer — real part models in their own colours, the routed copper on the board | built |

## Show a person, every time the geometry changes

A physical design is not reviewed from numbers. **After any change that moves
geometry, run `hardware/build.sh` and show the person the PNGs in
`hardware/build/review/`** (and offer `viewer.html`) before going further.
Say what changed and what set it: quote the lines of `layout.json → why` that
moved. Wait for their judgment when the change is a design choice — a size, a
placement, a look — and carry on when it was purely mechanical. A green
`checks.json` means it can be assembled, not that it is right.

## The model

| Concept | Is |
|---|---|
| **Outline** | any closed straight-edged SVG shape, by file, `layer` (`data-layer`), `path` index, `subpath` — convex or not |
| **Region** | another shape, in the same SVG coordinates |
| **Part** | a body (`body_mm`), a source (`mpn`, `lcsc` or a `pick`, `second_source`), a price, a `status`, a `place` and a `mount` |
| **Mount** | how the case holds it — the case features follow from it |

| `mount` | `place` | What the case gets |
|---|---|---|
| `surface` | a region | on top of the lid, bonded and sealed by an adhesive foam ring (`seal.adhesive_mm`, `adhesive_width_mm` — 3M VHB or equal); its leads go down a `pass_through_mm` hole inside the ring. `flush = true` sinks it into a pocket so its face is level with the lid top — the part drops in; the lid must be 1.5 mm thicker than the pocket |
| `window` | a region | the region cut through the lid as a window, a gasket round it, locator ribs holding the part behind it |
| `pocket` | a region | a pocket in the top of the lid |
| `socket` | `cavity` (beside the board) or `under-board` (on the floor beneath it) | four corner locators from the floor. Space is **reserved** at the declared envelope even while the part is `assumed`. Under the board, it may turn to fit between the standoffs, and the standoffs grow until the board clears it |
| `smd`, `tht` | `board` | nothing; its footprint sizes the board, its height the cavity |
| `pads` | `board` | nothing: copper pads, one per `nets` entry at `pitch_mm`, for a wire to land on — the EDA end of a `[[wire]]`. `follows = "supercap"` puts one pad under each lead of that socketed part instead, inset from the board edge the leads face, and `drill_mm` plates them through: the part lies flat and its leads solder straight into the board, no wire |
| `bulkhead` | `wall` | a hole through the longest edge facing `side`, clear of every screw boss |
| `vent` | a `case.mark`, or `lid` | a `pass_through_mm` hole through the lid and a membrane (`body_mm`) bonded over it inside; placed nearest its `near`, clear of the lid's other parts. With `seals_to = "<board part>"` it gets a **chimney**: a tube printed with the lid from round the membrane down to a silicone ring squeezed on the board round that part — outside air reaches the part, not the cavity. The part sits as near under the vent as it can — the tube may lean up to 3 mm (it still prints unsupported) — with its whole ring on the board and nothing else under it. The vent itself takes the nearest spot in its mark where the ring below fits the cavity; the board is held where the ring can land on it (a requirement, not a preference, on the floor) and sized for the ring, wires route round the tube, and the ring is on the BOM. When the ring cannot fit, derive names what it conflicts with. `membrane = "outside"` bonds the patch on the lid's face (sunk in a debossed mark), as adhesive vents are made to be: the bore then need only clear the hole and the part, so the ring is smaller. Under the ring the top copper is a keep-out for tracks and vias — the ring lands on flat pour, and the part's signals leave through vias inside the bore. `cad.py` checks the bore holds only that part, the lean, and that no wire passes through |
| `adhesive` | `lid`, or a mark | bonded flat to the lid's underside — a flex antenna, `near = "apex"` to reach up into the outline's highest point |

**Connections are declared once, on the parts.** A board part's `pins = {
VIN_DC = "SOLAR+", GND = "GND" }` maps a symbol pin name (every pin of that
name) or a pad number to a net. Those nets go onto the pads of
`board.kicad_pcb` and into `board.kicad_sch`; a pin that is not a pin of the
part's symbol, or a net that reaches only one pin, fails derive by name.
`value` is the component value for the BOM and schematic. `near =
"charger.VSTOR"` places a part beside that pin — decoupling where it belongs.
Wire pads (`mount = "pads"`) appear in the schematic as KiCad's generic
connector. `board.power_nets` names the nets that get wider tracks.

**Socketed parts can have leads**: `leads = ["+", "-"]`, `pitch_mm`, `lead_mm`
— out of the end that faces the board. A wire names a lead and a pad, `from =
"supercap#0.+"`, `to = "store-pads.1"`, and runs straight across between them.
A cable that comes with a part (an antenna pigtail) is `kind = "coax"` with
`fixed_mm`: the route must fit it, and nothing is cut. A wire runs square —
along one axis, a 90° bend, along the other — the way a soldered wire is laid.
A board part's `ports = { IPEX = [x, y, z] }` names a connector on it that has
no pad (a module's own U.FL), in footprint coordinates, so a wire can end on
`radio.IPEX`.

**Marks** on the lid: `[[case.mark]] region, piece = "roof", on, kind =
"deboss" | "emboss" | "etch", depth_mm` — the part of a region above its
full-width span (a house's roof), seated on a lid part's top edge. A vent can
sit in a mark. **Hanging**: `[case.hang] at = "top", hole_mm` — by default
(`kind = "holes"`) the outline's tip, `solid_mm` deep, is solid case with two
holes through it and no cavity behind them: one front to back, one across at
`cross_mm` for a nail from the side, so it hangs face-on or edge-on and stays
sealed. `kind = "lug"` with `width_mm, reach_mm, thickness_mm` is a tab out of
the highest point instead.

**How a region scales to its part** — `fit`, defaulted by the mount:

| `fit` | The region grows until… | Default for |
|---|---|---|
| `width` | its width equals the part's, exactly (not rounded) | `surface` |
| `height` | its height equals the part's | |
| `cover` | it meets the part less the window overlap — the part hides it | `window` |
| `inside` | the part fits inside it | `pocket` |

**The case's colour** — `case.colour = "#1d4ed8"`, the filament. The
renders and the viewer draw it, with the lid a shade lighter. Unset, it is a
neutral grey: no product's colour is anyone else's default.

**How the lid is held** — `case.fasteners.closure`:

| `closure` | Is |
|---|---|
| `screws-top` | screws down through the lid into bosses in the base |
| `screws-back` | screws up from underneath, through the base, threading into the lid — nothing on the face. The base grows if that is what lands a standard screw length with 1.5 d of bite and 1 mm of lid face left |
| `press-fit` | no screws: a skirt under the lid, `interference_mm` oversize. Friction holds the gasket's compression — prove it on a print |

**How the board is held** — `board.mount`: `screws` (default; four corner
screws into standoffs) or `snap` — two diagonal locating pegs through the
board's holes and a printed hook on each long edge that clicks over it. No
fasteners; prove the hooks on a print. `board.pegs = "bl-tr" | "tl-br"` picks
the diagonal, leaving free the corner a big part needs.

`count` sets how many screws (three is the fewest that hold a lid flat); they go
in the outline's convex corners, spread as far apart as possible, with bosses
big enough that the gasket groove runs **between** each screw and the cavity —
the screws are outside the seal.

**Wires** — `[[wire]] id, from, to` (a part id, or `part#n` for the n-th unit
of a `qty` part). Each is routed up to a corridor under the lid and down,
stepping out from under the board first where it must; its length is the
route plus `slack_mm`, plus `case.service_loop_mm` when an end is on the lid
so the lid opens without unsoldering. The cut length goes into the BOM, and
the assembly order solders what is under the board before the board goes in.
Wires are checked like parts, against every solid except what they are
soldered to; the corridor stays under the lid, round any part too tall to
clear, and a cradle's ribs stop short of a wire leaving its end.

**Components are KiCad's, not redrawn.** A board part names `footprint =
"Lib:Name"` and `symbol = "Lib:Name"` from KiCad's official libraries;
`python3 hardware/parts.py sync` vendors them, and the footprint's STEP model,
into `hardware/lib/` (hash-checked by `parts.py check`, tracked as derive
inputs). The part's size is then read from the courtyard (`dims_from =
"footprint"`), its pads land in `layout.json` where they are on the board, and
the render draws the manufacturer's model on its copper. A KiCad symbol that
`extends` another (most regulator variants do) is vendored whole: the
parent's pins and drawing under the variant's name and properties. When
KiCad's 3D library has no model for a footprint, the part renders as a box
built from its outline. `parts.py fetch <LCSC>` vendors the part's symbol,
footprint and model together from LCSC's library, made for one another. With no KiCad
component, declare `body_mm` and say where it came from: `dims_from`
(`datasheet`, `distributor`, `search`, `assumed`) and `source_url`. A
`decided` part whose size is not from a footprint, datasheet or distributor is
listed under `unverified_dimensions`.

**When the vocabulary runs out — the escape hatches.**

- `model = "hardware/vendor/x.step"` on any part draws its own solid model in
  place of its envelope (`model_rotate_deg` stands it up). The layout is
  still solved for `body_mm`; `cad.py` fails `models` if the model is bigger.
- `hardware/shapes.py` — the product's own file, never installed or
  overwritten — defines `shapes(layout, solids, kit)`. It runs after the
  build and before every check: add solids, cut or join the case. What it
  touched is recorded in `checks.json → custom`, and everything it made is
  checked like everything else.
- A bought board: `board.size_mm`, `board.holes_mm` from its drawing, and
  `at_mm` on each part — positions are facts there, not choices. Parts that
  hang past the board's edge take that room on the floor too.
- A socket that must be reached from outside: `through_wall = true` on a
  board part that `faces` an edge (or names `side`). It hangs past the board
  edge as far as its own pads allow (KiCad's 0.5 mm copper-to-edge, with a
  margin), and its mouth is its drawn body's face (F.Fab), so it comes flush
  with the case: derive fails if the mouth is more than 1 mm behind the
  outer face (`recess_mm` to allow more), naming the fixes. The window is
  the socket's face plus 1 mm each way; `opening_mm` sizes it for a plug's
  body instead and allows a 10 mm recess. A mouth inside the wall gets a
  notch open to the base's top, so the board drops in with the socket in
  it, and a plug on the lid fills the notch down to the window's top. A
  sealed case refuses the notch and keeps the socket behind the wall. Derive also fails if the window would cut a
  boss; the base rises so the window clears the lid seal. The window is not
  sealed, and `why` says so. The floor is placed by the solver to a tenth
  of a millimetre, so the board stands as close to that wall as it fits.

**Wires are dressed, not drawn.** Each `[[wire]]` is routed by a grid search
at its height: inside the cavity, round every boss, socket and part that
rises that high, with few, square turns. A multi-core wire's cores are true
parallel offsets of that route — they keep their sides and never cross — and
when it ends on a pads part with a pad per core, each core lands on its own
pad, approached square to the row, carrying that pad's net
(`layout.json → wires[].cores_mm`). A pad row nobody turned turns square to
its wire; a declared `rotate_deg` is followed and the route adapts. A
cylinder's free end is the end facing the other end of its cable.

**Set points are checked, not remembered.** A `[[check]]` drives nets
(`drive = { CC1 = { volts = 5.0, ohms = 56000 } }`: a voltage, behind a
resistance if given; GND is 0 V) and says where others must land
(`expect = { CC1 = [0.25, 0.61] }`). Derive solves it from the declared
resistors — every part whose `value` reads as one (`27R`, `5.1k`, `4k7`) and
whose two pins are on nets — by nodal analysis, and fails naming each net
outside its window; the results go in `why`. Change a resistor's value or a
pin's net and the check moves with it. It is DC and linear only: what
switches, saturates or moves in time (a regulator's loop, a transistor) is a
simulator's job, and no check can ask for it.

**One change, every discipline.** A net is declared once, on the pins of the
parts it joins. The same derive that puts it on the copper and in the
schematic writes it into `interface.ts` and, with `[firmware] mcu`, into
`board.rs`. The firmware includes that file (`#[path =
"../../../hardware/generated/board.rs"] mod board;`) and takes pins only
through it — never `p.PIN_4` by hand. Then moving a net to another pin moves
the firmware with no edit, and renaming a net breaks every line of firmware or
web code still using the old name, at compile time. Agents: when asked to
change a pin, change `pins` in `hardware/product.toml` and derive; do not
edit firmware pin numbers.

**Parts are packages: declare what a part must be, lock what it resolved to.**
A commodity part needs no catalogue number looked up by hand:
`pick = { category = "Resistors", package = "0603", value = "10k", has = ["1%"] }`
(also `mpn`, and `tier` = `basic` (default), `preferred` or `extended`, the
highest JLCPCB assembly tier allowed). `python3 hardware/parts.py resolve`
answers each pick from the JLCPCB catalogue (jlcparts): in stock for the
quantity built, lowest tier, then cheapest, then most stocked, and writes the
LCSC number, MPN, stock and price to `hardware/parts.lock`. Like a
`Cargo.lock`, it is committed and sticky: a locked pick is re-picked only when
the pick changes or `resolve --update` is run, so derive and CI never need
the network. `fid derive` fails while a pick is unanswered, and takes the
LCSC number from the lock into `bom.csv` and the fab files. A part is pinned
(`lcsc`) or picked, never both. An LCSC part that no KiCad library has —
most ICs — is vendored by `python3 hardware/parts.py fetch C2040`
(easyeda2kicad) into `hardware/vendor/lcsc.*`, then declared as
`footprint = "lcsc:<Name>"`. Goal: `build/fab/README.md` lists no unsourced
part, so the BOM is orderable as it stands.

**Every part carries its documents.** `datasheet = "<pdf url>"` on a part —
or, with none declared, the datasheet LCSC lists for its `lcsc` number — is
fetched by `parts.py sync` into `hardware/lib/datasheets/<MPN>.pdf`, with a
`.txt` copy beside it (pypdf), hashed in `manifest.json`. A datasheet the
network cannot reach is reported under `problems`, never guessed: save it as
`hardware/vendor/datasheets/<MPN>.pdf` and sync again. **Read the datasheet
before you choose a value, a pin or a land pattern** — grep the `.txt` first.

**`hardware/lib/PARTS.md` is the hardware's context.** One page, regenerated
by every sync and checked by `parts.py check`: per part, what it is, its
source and status, its symbol, footprint, 3D model and datasheet (as links),
and every pin's net. Start there when asked about the hardware; it is shorter
than `product.toml` and points at everything else. The bundle under
`hardware/lib/` is laid out as KiCad lays out its own libraries, so a person
can open it in KiCad, and a fab or another tool can read it, unchanged.

**The board** is sized from its parts at `board.fill`, under `board.max_mm`,
unless `board.size_mm` fixes it, and sectioned:

| Declare | Does |
|---|---|
| `board.zones = { rf = "top", power = "bottom" }` | bands along board edges (or `centre`). A band is as deep as its deepest part needs, `zone_depth` of the board at least |
| part `zone` | the part stays in that band |
| part `faces = "top"` / `faces_pin = "ANT"` | turns the part so that side, or the side that pad is on, faces the band's edge — RF out, toward the antenna |
| part `keepout_mm` | a copper-free strip from its facing side to the edge: a KiCad keep-out zone, and a film in the model that any part intruding fails |
| part `away_from`, `away_mm` | keeps it that far from those parts (heat, noise) |
| part `near` | a part id, a `part.PIN`, or a board side it sits toward — a **preference** (a cost), traded against short connections. Something that must be at an edge is a zone band (`zone`), not a `near` |
| part `rotate_deg` | turns it explicitly (90° steps) — a module's connector into a corner |
| `board.edge_mm` | a clear strip round the edge, no parts in it, where tracks run the perimeter; the board grows by twice it |
| `board.near = ["panel", "vent"]` | the board must reach under each of those lid parts (one, or a list), far enough in for the parts that are `near` it too — the panel's wire drops straight onto its pads, the sensor sits under its vent |
| `board.cut_mm = 2.5` | where the case narrows (a chamfer, a taper) the board keeps its length and loses its corners instead, each by at most this: its outline is the rectangle clipped to the cavity, wall and clearance in, written to `Edge.Cuts`. What was cut is an obstacle to every part (a part pinned there fails naming the corner), default holes move in with it, the case's solids and the insertion check use the outline. 0, the default: a rectangle |

**On the floor, placement is solved too** (the same solver). fid works out,
for each socket (either way round) and the board, the rectangles where its
centre may sit inside the cavity, its wall's distance in — any outline shape —
and writes them to `generated/floor-model.json`: every item and screw boss kept
`clearance_mm` apart, each as close to its `near` as the rest allows (the
board need only reach under its targets, and is centred on the first, other
things equal). The answer, `generated/floor.json`, is continuous to a tenth
of a millimetre and checked against the exact fit. `layout.json → why` says
how far each one ended up from where it asked to be; when they cannot all
fit, derive names which of them conflict.

**On the board, placement is solved by a constraint solver** (OR-Tools
CP-SAT, through `hardware/place.py`; spec 2026-10-01). Every rule in the
table above becomes data in `generated/placement-model.json`, one instance of
a generic kind:

| Kind | Hard or cost | From |
|---|---|---|
| inside | hard | `zone`, the board less `edge_mm` |
| no-overlap | hard | every part, 1 mm apart; the board's holes and snap hooks |
| fixed | hard | `at_mm`, `follows`; `rotate_deg` and `faces` fix the turn |
| flush | hard | `faces`: touching the zone's edge, its keep-out to the board edge |
| apart | hard | `away_from`, `away_mm` (edge to edge) |
| near | cost | `near` (a point, a part, or a pin's pad); otherwise a light pull to the middle of its zone |
| net | cost | every signal net's pads close together (half-perimeter); GND is a pour and does not pull |

A footprinted part nothing turns may take any quarter turn: the one that
shortens its connections wins. Explicit declarations are hard constraints,
so they always hold — or derive fails **naming the declarations that cannot
hold together** (a minimal set), with the model dumped to
`hardware/build/placement-model.json`.

The answer, `generated/placement.json`, is stamped with the model's hash and
committed. While the model is unchanged it is reused and no solver runs, so
`fid derive --check` needs no Python. A changed model needs `python3` with
`ortools` (`hardware/requirements.txt`) and takes about a minute:
`board.placement_effort` (10) trades time for shorter connections. The search
is deterministic: the same model gives the same answer.

Look at `build/review/ratsnest.svg` before routing: crossings you can see
there are tracks the router will struggle with.

## When it does not fit

Every failure names the part and the fact to change. Read it before touching
anything; it is usually one of: a part is too big for its region, the wall is
too thin for the seal, a hole is taller than the cavity, or a board part does
not fit at `board.fill`.

On the board, a failure is the solver's: "these declarations cannot hold
together" lists a minimal set — relaxing any one of them lets the rest hold.
Obstacles (the board's holes, the snap hooks) are named too. The model it
could not solve is in `hardware/build/placement-model.json`; run
`python3 hardware/place.py < hardware/build/placement-model.json` to try an
edit to it by hand before changing the declaration.

## Facts the electronics check for you

- **`value` is checked against the part ordered.** A pinned part's `value`
  ("1uF 50V X5R") must state the same resistance, capacitance or inductance as
  LCSC's own description of its `lcsc` number in `hardware/parts.lock`; a
  mismatch fails derive with both, so the schematic, the BOM and the reel
  agree.
- **`[rails]` and `io_max_v`.** Declare each rail's voltage
  (`[rails] VBUS = 5.0, "3V3" = 3.3`) and, on a part, the highest voltage its
  pins may see from its datasheet's absolute maximum ratings
  (`io_max_v = 3.8`). Every net a resistor pulls toward a rail is solved, and
  a pin above its part's limit fails derive, naming the net and the resistor
  that put it there.
- **`i2c_address`.** A part's I²C address (`i2c_address = 0x38`) is declared
  on the part and derived into `board.rs` as `<PART>_I2C_ADDRESS`. Firmware
  takes it from there: an address typed into an I²C transfer
  (`write_async(0x38, …)`) fails derive, unless marked `// fid: allow-address`.
- **Device profiles: the datasheet, read once.** A part whose `mpn` the
  platform has a profile for (RP2040, AHT20, SN74AHCT1G125, WS2812B so far) is
  checked against its datasheet on every derive: its supply pin's rail must be
  in range; its `io_max_v` may not exceed the datasheet's (an RP2040 on 3.3 V
  is 3.8 V, not 5.5 — the limit is not a setting); its `i2c_address` must be
  the datasheet's; an active-low enable tied high (or active-high tied low) is
  a part that never turns on. What you leave out, the profile fills in. Its
  command bytes reach `board.rs` as `<PART>_<COMMAND>` (`SENSOR_INIT`); a
  command written out by hand in firmware (`const INIT: [u8; 3] = [0xE1, …]`)
  or typed into a transfer (`write(addr, &[0xE1, …])`) fails derive.
- **Every I²C line is pulled up.** A net on a pin called SDA or SCL needs a
  resistor to a supply rail; a bus line nothing pulls high never reads high.
  A copied pull-up that kept its source's net leaves its own line bare, and
  derive names it.
- **Pins go through `board.rs`.** Firmware that names a pin directly
  (`p.PIN_4`, `p.PA5`) fails derive and `--check`, naming file and line; a
  deliberate one is marked `// fid: allow-pin`.

## Drift you cannot see

The declaration names SVG files it does not own — a logo belongs to the brand.
Their hashes go into `fiducial.lock` as **inputs**, so a redrawn logo fails
`fid derive --check` naming the file, and `layout.json → shapes_read` records
the vertex count, bounds and area of each shape read, so the redraw shows up
in review as a diff. Never silence that failure with a bare `fid derive`:
look at what moved, re-render, and show it.

## What is not derived

- **Routing is a search, not a derivation**, so it is built, not gated:
  `route.py` routes `board.kicad_pcb` with Freerouting (fan-out first)
  into `build/board-routed.kicad_pcb`. A part without a footprint is a
  courtyard placeholder (`Pending_<id>`) until it gets one.
- **A board the autorouter cannot do, or should not** (fast signals, RF, a
  layout a person wants to own): `board.routing = "hand"`. Copy
  `generated/board.kicad_pcb` to `hardware/board-routed.kicad_pcb`, route it
  in KiCad and commit it. It stays gated: `fid derive` fails while any part
  in it has moved, turned, flipped, been added or removed, or any pad is on
  another net than the declaration says, or the outline differs, naming
  each. Change `product.toml`, re-derive, and re-route what moved; never
  move a part in KiCad. The build runs no router: it applies the declared
  rules and KiCad's DRC to that board.
- **A component no library has.** Put it in `hardware/vendor/` (KiCad's own
  layout: `<Lib>.pretty/`, `<Lib>.kicad_sym`, `<Lib>.3dshapes/`) with a
  `SOURCES.md` naming where every number came from — a land pattern from a
  board that uses the part, a symbol from an openly licensed library. A
  footprint with no model gets a body built from its own fab outline.
- **Curves.** Straight segments only; a curve fails naming the command.
- **Prices.** Never estimated. `resolve` records each pinned part's LCSC
  price at the build quantity too, minimum orders included and pooled across
  lines of the same part, and derive asserts the total against
  `cost.ceiling`. An unpriced line (a printed part, an assumed one) is
  listed; `[cost.prices]` declares one; `cost.strict = true` makes any left
  unpriced fail derive.

## Tools

`python3` + `pip install -r hardware/requirements.txt` (build123d on
OpenCascade: real B-rep, real STEP), KiCad ≥ 7 (`kicad-cli`, and its
`pcbnew` Python module, which `route.py` runs under `/usr/bin/python3`), Java
17+ for Freerouting (fetched once into `build/tools/` and pinned by hash), and `node` with
`three` and `playwright-core` for the renders; set `CHROMIUM_PATH` if
Playwright has no browser of its own. None of them is needed by the gate.

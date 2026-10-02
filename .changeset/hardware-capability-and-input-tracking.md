---
"@fiducial/cli": minor
---

`fid add capability hardware`: a physical product declared once.

`hardware/product.toml` names an outline and regions in any SVG (by
`data-layer`, path and subpath — non-convex shapes included), and parts with
bodies, sources, prices, a place and a mount. The new `fid-hardware` executor
solves it: the scale grows until each window part meets its region, the board
is sized from its parts, sockets and board are packed together, lid screws go
in the convex corners with bosses the gasket groove routes round, and wall
parts get holes clear of them. It writes `layout.json` (with a `why` list),
`bom.csv` (including the screws and gaskets the design implies),
`assembly.md` and a KiCad board frame — all gated by `fid derive --check`.
A priced BOM over its ceiling fails derive.

The capability also installs `hardware/cad.py` (build123d/OpenCascade: STEP
and STL for every part, and interference, insertion, lid-closing and screw
checks that fail the build) and `hardware/render.mjs` (four review renders and
a self-contained 3D viewer).

Also in the vocabulary: `surface` parts on top of the lid, bonded by an
adhesive foam ring with a lead hole inside it; region scaling as a `fit` rule
(`width`, `height`, `cover`, `inside`); parts `under-board`, with the
standoffs raised to clear them; `[[wire]]`s routed between parts with their
cut length and a lid service loop in the BOM; and `closure = screws-top |
screws-back | press-fit`, where screws from the back raise the base if that is
what lands a standard screw length.

Components are KiCad's own: a board part names a `footprint` and `symbol`,
`hardware/parts.py sync` vendors them with their STEP models, and the part's
size, pads and model come from the footprint. Boards are sectioned into
`zones` (RF at an edge, turned by `faces`/`faces_pin`, with a `keepout_mm`
zone), `pads` parts carry wire nets into `board.kicad_pcb`, and every size not
from a footprint says where it came from (`dims_from`, `source_url`). The
gasket stands proud of its groove and `checks.json` verifies the lid squeezes
it. Placement is a bounded best-cost search over every `near` preference.

`fid upgrade` installs files a capability gained after the product installed
it, leaving a same-named file of the product's own alone.

**Connected, routed, drawn as built.** Board parts declare `pins = { VIN_DC =
"SOLAR+" }` by symbol pin name; the nets go onto the pads of
`board.kicad_pcb`, and a new derived output, `board.kicad_sch`, places each
part's KiCad symbol with a net label on every pin — byte-stable, so it is
gated. A pin the symbol lacks, or a net with one end, fails derive.
`hardware/route.py` (new) routes the board with a pinned Freerouting, pours
ground on both layers and runs KiCad's DRC, failing the build on anything
unconnected. `parts.py` reads a product's own `hardware/vendor/` libraries
first; the renders show each STEP model in its own colours, build a body from
the footprint where no model exists, and draw the routed copper.

Socketed parts get `leads`, and wires name a lead and a pad (`supercap#0.+` →
`store-pads.1`). The lid gets `[[case.mark]]` (a region's roof, debossed onto
a lid part), `vent` and `adhesive` mounts, and `[case.hang]`; `board.near`
takes a list.

**Pipelines now track their inputs.** A pipeline may list `inputs`, and a
built-in executor reports what it reads; their hashes go into
`fiducial.lock [inputs]`, and `fid derive --check` fails naming the upstream
file that moved — a redrawn logo, say — even when every output still matches
its own hash. The table is omitted when empty, so existing locks do not change.

A board can go together with no wires and no screws: `pads` with `follows`
and `drill_mm` put plated holes under the leads of a part lying beside the
board, `board.mount = "snap"` holds it on two pegs and two printed hooks, and
a wire can end on a `ports` connector (a module's own U.FL). `rotate_deg`
turns a part; decoupling `near` a pin is placed out from that pad; `flush`
sinks a surface part level with the lid top; and `case.hang` defaults to two
holes through a solid tip, face-on and edge-on, with the seal unbroken.

Board parts are placed by a constraint solver: `fid-hardware` writes the
problem as `generated/placement-model.json`, `hardware/place.py` solves it
with OR-Tools CP-SAT, and the answer, `generated/placement.json`, is stamped
with the model's hash and reused while the model is unchanged — so
`fid derive --check` needs no Python. Every rule is an instance of a generic
kind; a contradiction fails naming the declarations that cannot hold
together. A vent `seals_to` a board part gets a chimney: a tube printed with
the lid, sealed on a ring round that part. A product's own geometry goes in
`hardware/shapes.py`, a part's own STEP in `model`, and wires are routed
round every obstacle.
`board.edge_mm` keeps a perimeter strip clear for tracks, `board.pegs` picks
the snap pegs' diagonal, and `board.track_mm` / `board.clearance_mm` set the
routing rules. `hardware/ratsnest.py` draws the placement and its ratsnest
into the review pack. Every part can carry its `datasheet`: `parts.py sync`
fetches it (or the one LCSC lists) with a text copy, and writes
`hardware/lib/PARTS.md`, every part on one page, the hardware's context for
a person or an agent.

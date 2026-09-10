# Connector cutouts, standoffs, and how a punched face stays closed

**Date:** 2026-09-10
**Status:** Accepted
**Supersedes:** nothing. Closes two of the gaps recorded in
`2026-09-08-phases-13-15-transports-eda-geometry.md`.

The sealed case shipped as a box with no holes in it. Connector positions were
declared and nothing consumed them, so the geometry was not usable for a
product. This records how openings are declared and cut, and what is still
missing.

---

## 1. A connector's opening is declared on the board, not on the wall

`connector.mount` carries `side` and `offset_mm` in board coordinates:
`offset_mm` runs along the named edge from the board's origin corner, on every
side.

**Why not wall coordinates.** The wall's thickness is derived — the seal sets it
(decision 10 of the previous spec). A declaration in wall coordinates would
therefore shift every time the gasket stock changed, and the author would have
to work the position out backwards from a number they did not choose. Board
coordinates are the ones a layout tool already knows.

**The consequence.** North and West walls run against the board axes, so the
resolver reverses them. That arithmetic lives in one function
(`Case::hole_for`) rather than in the declaration, which is the point: a
connector 20 mm along the south edge and one 20 mm along the north edge sit at
the same board *x*.

## 2. `mount` is optional per connector, and absence means no hole

**Why absence rather than a flag.** An opening is a leak in a sealed case. The
default has to be the safe one, and the safe one is no hole. A debug header
reached with the lid off is the common case, not the exception.

The seed board demonstrates this: it mounts its USB-C and Qwiic ports and
deliberately leaves its SWD header unmounted. At 8.5 mm tall, mounting it would
need more than 11.7 mm of headroom on the FDM profile — the body, two process
tolerances, the groove depth, and the edge margin — where the seed declares 10.
That is a case you could build; it is not the case the seed describes, and the
header is reached with the lid off.

## 3. The declared size is the connector body; the process adds the clearance

`mount.width_mm` / `height_mm` — and the family envelopes they default to — are
body dimensions. The opening is the body plus one process tolerance on every
side.

**Why split it that way.** A connector's body is a fixed fact about a part; the
clearance it needs is a fact about the printer. Keeping them separate means the
same declaration cuts a tighter hole on resin than on FDM, exactly as
`tolerance` already does for the shell. Declaring the *opening* instead would
bake one process into the board file.

## 4. `type` implies the envelope, from a table in `fiducial-geometry`

`CONNECTOR_OPENINGS` sits beside `TOLERANCE_FDM` and friends, and
`fiducial-eda` takes a dependency on the geometry crate to validate against it.

**Why there rather than in the EDA crate.** It is a table of physical
dimensions, which is what that crate is for, and it is consumed by the code
that generates geometry. Putting it in the declaration crate would mean the
mesh had to ask the declaration what a millimetre was.

**Why validate an unlisted family as an error.** A guessed envelope produces a
case that prints, looks right, and does not fit the connector. Failure at the
declaration is the only cheap failure. `fid derive` names the type and says to
declare `width_mm` and `height_mm`.

## 5. A punched face keeps a mitred frame around an inset grid

Each wall face is tessellated as four corner-fanned frame polygons plus a grid
of cells with hole cells omitted. The tunnel through the wall and the standoff
posts are subdivided on the same lines.

**The problem this solves.** Subdividing a face's edge breaks the surface beside
it: the neighbour still has one edge where the punched face now has three, which
is a T-junction, and a T-junction is not a closed manifold however watertight it
looks. Propagating the subdivision outward would mean the rim ring, both groove
bands, the lip ring, and the cavity floor all had to know which walls had holes
in them.

**Why the frame fixes it.** The fan lets the *inset* boundary carry arbitrarily
many vertices while the *outer* boundary stays four single edges. Features stay
local: the rim, groove, and lip rings are generated exactly as they are for a
featureless case, and no surface downstream of a wall knows a hole went through
it.

**What it costs.** A punched face is more triangles than a quad, and the frame
width is a tessellation parameter (half the process minimum feature) that
constrains where a hole may sit. A face with no holes short-circuits to a single
quad, so an unfeatured case is byte-identical to one generated before cutouts
existed — asserted by a test, because that equality is the whole argument that
this change is additive.

## 6. Standoffs are punched out of the floor, not dropped onto it

Declaring `standoff_height_mm` punches four footprints in the cavity floor and
continues the surface up each post.

**Why not four separate boxes merged in.** Slicers union overlapping solids, so
separate posts would print. But the result is not a closed manifold, coincident
faces produce slicing artifacts, and — most importantly — the base's
watertightness test would have to be weakened to accept it. Making the posts
integral keeps one assertion covering the whole part.

**Why the surface direction works out.** The floor cap faces up, so material is
below it. At a hole rim the surface turns up the post's outer wall, where
material is inside, and closes with an upward-facing top cap. Orientation stays
consistent all the way round, which the directed-edge test verifies.

## 7. Standoffs raise the rim rather than eating the headroom

`z_rim = floor + standoff + thickness + headroom`.

**Why.** Headroom is declared as space above the board. If posts lifted the
board without lifting the rim, declaring standoffs would silently shorten the
component clearance the author asked for — and the failure would surface as a
lid that will not close.

## 8. Watertightness is now asserted by directed-edge uniqueness and volume sign

The mesh tests require every *directed* edge to occur exactly once, its
opposing twin to exist, and the signed volume to be positive.

**Why strengthen it.** Undirected parity (decision 8 of the previous spec) is
satisfied by two adjacent faces wound the same way, which reads as a crease with
material on both sides, and it does not catch a globally inverted solid.
Directed uniqueness catches both, and it is exactly the property T-junctions
violate — the failure mode the frame-and-grid tessellation exists to prevent.
The volume sign is the one check that a slicer's own repair pass cannot mask.

## 9. Validation is separate from generation, and reports in the declaration's terms

`Case::validate()` returns a `CaseError`; the mesh functions are infallible.

**Why not fail inside the generator.** Every rejected condition — an opening
that reaches the groove, one that runs off its wall, two that merge — produces
geometry a slicer accepts. There is no downstream point at which the error is
still detectable, so the declaration is the last place it can be explained. The
messages name the connector and say which declared field to change.

**Why the meshes stay infallible.** Generation is then a pure function of a
validated declaration, and the CLI has one obvious place to check. The cost is
that a caller who skips `validate()` gets a leaky case; the doc comment says so.

## 10. The lid never takes features

**Why.** A hole in the lid is a hole inside the gasket line. Compression cannot
seal it, so an opening there is not a trade-off — it is a mistake with no
correct configuration. Retaining fasteners have the same problem, which is why
they are still missing rather than placed somewhere convenient (see below).

---

## Known gaps

Carried forward and revised. The previous spec's gaps for cutouts and standoffs
are now closed.

- **The lid is still not retained.** Nothing clamps it down, so the gasket is
  only compressed while something external holds the lid closed. Doing this
  properly needs fasteners *outside* the gasket line — either external corner
  ears or a round hole through the outer lip — and both need geometry this
  crate does not have yet: polygon offsetting for the ears, circle tessellation
  in a punched ring for the holes. A square pocket where a screw belongs would
  be worse than the honest gap.
- **The case is not pressure-rated.** A compression gasket resists splashes and
  dust. Nothing here has been tested to an IP rating and the geometry claims
  none.
- **Openings are rectangular only.** Fine for every family in the table; a
  round barrel-jack or SMA bulkhead gets a square hole slightly larger than it
  needs. Same missing primitive as the fastener holes.
- **Standoff positions are derived, not declared.** Four posts, one under each
  board corner, inset by the process minimum feature. A board whose mounting
  holes are elsewhere cannot say so. Deriving them is right until a product
  needs otherwise (Rule of Two, §16).
- **`Polygon::signed_area()` still has no consumer.** It exists for the winding
  checks that offsetting a non-rectangular outline will need — the same
  primitive the two gaps above are waiting on.
- **`fid derive --check` detects tampering with an artifact, not staleness
  against an input.** Unchanged: it works for the board today only because
  `board.interface.json` is itself a tracked artifact of the `eda` pipeline.

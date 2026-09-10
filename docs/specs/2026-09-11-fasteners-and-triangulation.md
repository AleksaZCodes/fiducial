# Fasteners, and the triangulation primitive they needed

**Date:** 2026-09-11
**Status:** Accepted
**Supersedes:** nothing. Closes the "lid is not retained" gap recorded in
`2026-09-10-connector-cutouts-and-standoffs.md`.

The case sealed only while something external held the lid shut, because
nothing clamped it. Fixing that needed a primitive the crate did not have.

---

## 1. It was triangulation that was missing, not offsetting

The previous spec said the remaining gaps waited on polygon offsetting. That was
wrong, and worth correcting because it points at the wrong file.

Offsetting is needed to inset a *non-convex* outline. Everything actually
outstanding needed something else: a way to triangulate a face with holes in it.
A rectangular annulus is `rect.inset()`, which is exact for rectangles; the
missing piece was putting a round hole through one.

## 2. A fastener hole sits on the mitre, so the rim must be one polygon

`MeshBuilder::ring` emits four mitred trapezoids. A corner fastener sits at the
midpoint of the outer lip in both axes, which puts its centre exactly on the
mitre diagonal — so the hole straddles two trapezoids and cannot be punched
piecewise.

`flat_face` triangulates the whole annulus as one polygon instead. It has no
mitre to straddle, and it preserves both boundaries, so the outer band and the
groove band below need no change at all.

**Why not move the fastener off the mitre.** Corner screws are what stop a lid
bowing in the middle of a long side, and the corner is where the outer lip is
widest. Placing screws at edge midpoints to dodge a tessellation detail would be
letting the implementation pick the mechanical design.

## 3. Bridge selection is exact, not heuristic

Ear clipping a polygon with holes needs each hole bridged into the outer loop.
The textbook method — and earcut's — ray-casts left from the hole and picks a
target by a tangent heuristic.

**Why that was not good enough.** It can choose a vertex the hole cannot see.
The bridge then runs through another hole, the merged ring stops being simple,
ear clipping runs out of ears, and the result is a surface with a chunk
missing — which still slices, still looks plausible, and is wrong. Faces here
have tens of vertices, so testing visibility exactly costs nothing that matters.

Three further corrections were needed, each found by a fuzz case rather than by
reading:

- **Candidate bridges are tested against holes not yet merged.** Validating
  against the ring alone accepts a bridge that only becomes an intersection
  later. The corner fastener case is exactly this: the hole sits on the diagonal
  the annulus wants to bridge along.
- **A vertex already carrying a bridge cannot be a second bridge's target.** A
  bridge is a zero-width slit that pinches the polygon at both ends. Where three
  met, a triangle spanning the pinch looked valid edge-by-edge while being
  topologically wrong, and clipping it corrupted the ring.
- **An ear needs a valid diagonal**, not merely a convex corner with no vertex
  inside it. Without that test the clip cuts straight across a bridge.

**What this buys.** The primitive is verified by area conservation, per-triangle
winding, and boundary-edge equality against the input loops, over 3000 fuzz
cases plus swept annulus rims and case walls. Boundary-edge equality is the
load-bearing one: it is the property that keeps a punched face flush with its
unpunched neighbours.

## 4. Screws pass through the outer lip, and that sets the wall thickness

A clearance hole in the lid, a pilot hole in the base, both through the lip
outboard of the gasket groove.

**Why outboard.** A hole anywhere inside the gasket line opens the sealed cavity
to the outside, and the case still prints and slices perfectly. There is no
configuration of an inboard fastener that is correct.

**Why the base hole is undersized.** A clearance hole in both parts would give
the screw nothing to grip; a pilot in both would stop the lid being pulled down.
The pilot is 0.8 × shaft, the usual ratio for a screw tapping its own thread in
plastic.

**The cost, stated plainly.** The lip must carry the hole with a printable wall
either side, so a 3 mm screw on the FDM profile widens the outer lip from 1.2 mm
to 5.87 mm and the wall from 4.8 mm to 9.47 mm — a 100 × 60 board goes from a
110 × 70 case to 120 × 80. That is why fasteners are opt-in: a case that does
not need a retained lid should not pay for one. The seal itself is untouched;
only the lip outboard of it moves.

## 5. The wall is sized to the hole that gets cut, not the one asked for

A regular polygon approximating a circle is *inscribed*, so it is narrower than
the circle across its flats and wider across its corners.

Both facts matter, in opposite directions. `circumradius_for_width` scales the
construction radius by 1/cos(π/n) so the flats reach the diameter asked for —
without it a 3 mm screw would not pass a hole labelled 3 mm. And the lip is
sized from the resulting *corner* extent, not the nominal diameter: sizing to
nominal left 1.17 mm of wall where the process minimum is 1.2 mm. A quarter of a
millimetre, on the wall that holds the lid down.

## 6. An unfastened case is byte-identical to one from before fasteners existed

Asserted on the STL bytes, as it is for cutouts. Every punched surface
short-circuits to its old form when there is nothing to punch.

**Why assert it rather than reason about it.** It is the whole claim that this
change is additive. Widening a lip and re-tessellating a rim are exactly the
kind of changes that quietly alter every case that never asked for a screw.

---

## Known gaps

- **Fastener positions are derived, not declared.** Four screws, one per corner,
  centred in the outer lip. A product wanting six on a long case cannot say so.
  Deriving them is right until a real product needs otherwise (Rule of Two,
  §16).
- **No countersink or boss.** The screw head sits proud of the lid. A counterbore
  is a second concentric hole at a shallower depth — straightforward now the
  primitive exists, but nothing has asked for it.
- **Openings are still rectangular only.** A round barrel jack or SMA bulkhead
  gets a square hole slightly larger than it needs. The primitive to fix this now
  exists — `flat_face` takes any loop, and `circle` generates one — so this is
  now a small change rather than a missing capability.
- **The case is still not pressure-rated.** A compression gasket resists splashes
  and dust. With the lid now actually clamped the seal is real, but nothing here
  has been tested to an IP rating and the geometry claims none.
- **Standoff positions are still derived**, and `Polygon::signed_area()` still has
  no consumer — it waits on offsetting a non-rectangular outline, which remains
  the one genuinely absent primitive.

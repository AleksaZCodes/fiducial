# Phases 13–15 — browser transports, EDA declaration, generated geometry

**Date:** 2026-09-08
**Status:** Accepted
**Supersedes:** nothing. Extends `2026-09-06-fiducial-design.md`.

Decisions taken while building the web transport layer (Phase 13), the EDA
pipeline (Phase 14), and the geometry/mesh pipeline (Phase 15). Recorded per the
append-only rule: later reversals are appended, not edited in.

---

## 1. The browser codec is a hand-written port, not compiled from Rust

`packages/transport-web/src/codec.ts` reimplements `fiducial-protocol` in
TypeScript rather than reusing it through WASM.

**Why.** The frame codec is roughly forty lines of branch-free byte handling. A
WASM boundary would add a build step, an async init, and a payload copy per
frame to share it — cost out of proportion to the code saved. The codec is also
the one part of the protocol that must run inside a `characteristicvaluechanged`
callback, where an uninitialised WASM module is a failure mode with no good
recovery.

**The risk this accepts.** Two implementations can drift. The mitigation is that
the frame format is frozen by the wire, not by either implementation: any drift
shows up as a device that cannot talk to a browser. Both sides are tested
against the same fixtures.

**Reverse this if** the protocol grows stateful negotiation or compression. At
that point the duplicated logic stops being a page of byte-shuffling and the
WASM boundary becomes the cheaper option.

## 2. Decoder buffers are fixed-size and drop oversized frames silently

`FrameDecoder` allocates a fixed buffer (default 4096 bytes) and discards any
frame declaring a longer payload, resynchronising on the next magic byte.

**Why.** It mirrors the `no_std` `FrameDecoder<N>`, which cannot grow a buffer at
all. Matching the firmware's behaviour means a frame that a device cannot decode
also fails in the browser, rather than the browser silently accepting traffic the
hardware would reject.

**The trap this creates.** `encode()` permits payloads up to 65535, so the
encoder and the default decoder disagree. Every transport therefore exposes
`maxPayload`, and the README states the mismatch explicitly. The alternative —
defaulting to a 64 KB buffer per transport — spends memory on every connection
to hide a limit callers should know about.

## 3. Default BLE UUIDs are placeholders and are documented as such

`fd01`/`fd02`/`fd03` are in the 16-bit SIG range and are not allocated to
Fiducial.

**Why not just leave them.** BLE UUIDs cannot be changed after devices ship
without breaking every existing pairing, which makes an accidental default the
most expensive kind of default. `BleOptions` takes `serviceUuid`, `txUuid`, and
`rxUuid`; the constants stay as documented development defaults.

## 4. `board.interface.json` is the single board declaration

Connectors, pins, net classes, and the physical outline live in one generated
file, hashed in `fiducial.lock`.

**Why one file.** Splitting the electrical interface from the physical outline
would let the two drift — an enclosure sized for a board revision the netlist no
longer describes. One declaration means one hash, and `fid derive --check`
catches a hand-edit to any part of it.

**Schema evolution.** `outline` is optional and its sub-fields default, so
adding it did not bump `schema_version`. A field whose absence has a sound
default is additive; a field that changes the meaning of existing data is not
and requires a version bump.

## 5. Validation rejects degenerate geometry at the declaration

`validate()` fails a non-positive `width_mm`, `height_mm`, or `thickness_mm`, and
an unrecognised `tolerance`.

**Why here rather than in the mesh code.** A zero-width board produces a
degenerate but structurally valid STL — slicers accept it and print nothing. The
failure surfaces hours later at the printer with no trace back to its cause. The
declaration is the last point where the error is still cheap to report.

## 6. Enclosure dimensions derive from tolerance profiles, not constants

`EnclosureParams::from_outline()` computes clearance as 2 × the process XY
accuracy, and wall and floor thickness from the process minimum wall thickness.

**Why 2×.** The board edge is manufactured to one tolerance and the printed wall
to another; both can drift toward each other. A clearance of one tolerance fits
on average and binds half the time.

**What this buys.** The tolerance class stops being metadata. Changing `fdm` to
`resin` in the declaration regenerates a measurably tighter enclosure, which is
the property that makes the profiles worth having.

`headroom` is the exception — 5 mm with no process basis, since component height
is not a property of the manufacturing process. It is documented as a value to
override rather than one derived from anything.

## 7. `fid-mesh` and `fid-validate` run in-process

Both executors are Rust functions inside `fid`, not shelled-out tools.

**Why.** CI can then verify the whole chain without atopile, KiCad, or a CAD
kernel installed. A pipeline that only runs where a proprietary tool is present
is a pipeline that stops being checked. The cost is that `fid` links the geometry
crates; they are `no_std` and small, so the binary grows by little.

## 8. Watertightness is asserted by edge parity, not triangle count

Mesh tests check that every undirected edge is shared by exactly two triangles.

**Why.** A triangle count confirms the generator emitted the expected number of
faces; it says nothing about whether they form a closed solid. Edge parity is
the definition of a closed manifold and is what a slicer actually requires.
Vertices are quantised to 1 µm before comparison because faces duplicate their
corners to keep flat normals.

## 9. Rectangular outline, polygon-shaped seam

`BoardOutline` stores width and height, but enclosure sizing reads
`to_polygon().bounding_box()`.

**Why the indirection.** Real boards have cutouts and rounded corners. Routing
today's rectangle through `Polygon` means the non-rectangular case becomes a
change to `to_polygon()` and the offset logic, rather than a change to every
consumer of `width_mm`/`height_mm`. The cost is one allocation per generation,
which is not on any hot path.

---

## 10. The case seals with a compression gasket, printed as a separate part

The generated enclosure is a base with a grooved rim, a gasket ring, and a lid
with a matching tongue. Three files, not one.

**Why three parts rather than a printed-in seal.** The seal has to compress and
the shell has to be rigid — one object cannot be both. Splitting them lets the
base and lid print in PLA/PETG/ABS and the gasket in TPU, on the same printer,
without multi-material hardware.

**Why a groove rather than a flat face gasket.** A flat gasket squeezed between
two flat rims squirms out sideways under compression. A groove captures it, so
compression goes into sealing instead of extrusion.

**Why the gasket seats flush and the tongue does the squeezing.** Groove depth
equals gasket height, and the tongue penetrates by
`gasket_height × gasket_compression`. That makes compression a single dimensionless
knob with an obvious physical meaning, instead of three interacting depths.
Compression is rejected at 0 (never squeezes) and 1 (crushes flat).

**The consequence worth knowing.** The seal now sets the wall thickness: a wall
must carry a lip, the groove, and a second lip. Sealed cases therefore have
visibly thicker walls than the plain tray, and `wall_mm` is derived rather than
configured.

## 11. `outline.enclosure` carries only what the process cannot imply

Headroom, lid thickness, and gasket cross-section are declared. Clearance, lip
width, floor thickness, and gasket fit are derived from the tolerance class and
are deliberately *not* overridable there.

**Why the split.** A manufacturing process implies how accurately it can hold a
dimension; it cannot know how tall your tallest capacitor is. Letting the
declaration override process-derived values would let a product silently
specify a wall thinner than the printer can produce.

## 12. Mesh outputs are addressed by stem and extension independently

`enclosure/case-base.stl` and `enclosure/case-base.glb` are the same geometry in
two encodings; the stem selects the part, the extension the format.

**Why not a `parts = [...]` key.** Outputs already have to be listed for
`fiducial.lock` to track them. A separate parts list would be a second place to
declare the same thing, and the two could disagree. An unknown stem fails the
pipeline and prints the valid set.

**What this buys.** Writing the preview into a web app is adding one path to
`outputs` — no copy step, and the copy is hash-tracked like every other
artifact, so `--check` catches it going stale. It ships commented out, because a
product with no web app should not get a stray file.

---

## Known gaps

Recorded so they are not rediscovered as surprises.

- **No connector cutouts, standoffs, or fastener bosses.** Connector positions
  are declared in `board.interface.json` but nothing consumes them
  geometrically. Cutouts are the obvious next derivation, and the one that makes
  the case usable for a real product rather than a sealed box.
- **The case is not pressure-rated.** A compression gasket resists splashes and
  dust. Nothing here has been tested to an IP rating, and the geometry makes no
  claim to one.
- **The lid is not retained.** Nothing clamps it down — no screw bosses, clips,
  or latches — so the gasket is only compressed if something external holds the
  lid closed.
- **`Polygon::signed_area()` has no consumer.** It exists for the winding checks
  that offsetting a non-rectangular outline will need.
- **`fid derive --check` detects tampering with an artifact, not staleness
  against an input.** It works for the board today only because
  `board.interface.json` is itself a tracked artifact of the `eda` pipeline. A
  pipeline whose inputs are not themselves tracked would not get the same
  protection.

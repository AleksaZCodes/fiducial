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

## Known gaps

Recorded so they are not rediscovered as surprises.

- **The enclosure is a tray, not a case.** No lid, standoffs, connector cutouts,
  or fastener bosses. Connector positions are declared in
  `board.interface.json` but nothing consumes them geometrically yet — that is
  the obvious next derivation.
- **`Polygon::signed_area()` has no consumer.** It exists for the winding checks
  that offsetting a non-rectangular outline will need.
- **The marketing page is a capability template.** It expects
  `enclosure/board.glb` to be copied into the app's public directory; no build
  step automates the copy.
- **`fid derive --check` detects tampering with an artifact, not staleness
  against an input.** It works for the board today only because
  `board.interface.json` is itself a tracked artifact of the `eda` pipeline. A
  pipeline whose inputs are not themselves tracked would not get the same
  protection.

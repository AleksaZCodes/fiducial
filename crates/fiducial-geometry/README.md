# fiducial-geometry

**Geometry that knows what process will make it.**

[![crates.io](https://img.shields.io/crates/v/fiducial-geometry.svg)](https://crates.io/crates/fiducial-geometry)
[![docs.rs](https://docs.rs/fiducial-geometry/badge.svg)](https://docs.rs/fiducial-geometry)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

`no_std` 2D/3D primitives for the board → enclosure pipeline, plus the two things
that make the pipeline more than arithmetic:

**Tolerance profiles.** `ToleranceProfile` carries per-process XY accuracy and
minimum wall thickness for FDM, resin and CNC. Downstream geometry derives its
clearances from the profile, so the *same* board yields a tighter case on resin
than on FDM without anyone editing a dimension. The manufacturing process is a
declared fact, and the geometry is derived from it.

**Connector envelopes.** `CONNECTOR_OPENINGS` maps a connector family
(`usb-c`, `qwiic`, `swd`, …) to the body envelope it needs. A board that declares
`"type": "usb-c"` therefore implies the size of the hole in the case wall — the
opening is never typed in a second time.

## Install

```sh
cargo add fiducial-geometry
```

## Use

```rust
use fiducial_geometry::{BoardOutline, ToleranceClass, connector_opening, Side};

// A 100 × 60 mm board, to be printed on FDM.
let outline = BoardOutline::new(100.0, 60.0).with_tolerance(ToleranceClass::Fdm);
assert_eq!(outline.width_mm, 100.0);

// The declared connector type implies its opening — no second declaration.
let usb = connector_opening("usb-c").expect("usb-c is a known family");
assert!(usb.width_mm > 0.0);

// Sides are named, so an offset means the same thing on every edge.
assert_eq!(Side::from_name("south"), Some(Side::South));
```

## Triangulation

`triangulate` is ear clipping of a polygon **with holes**, using *exact* bridge
visibility rather than the ray-cast/tangent heuristic earcut uses. It is verified
by area conservation, per-triangle winding, and boundary-edge equality against the
input loops over 3000 fuzz cases. Three separate bugs found during that fuzzing
each silently dropped part of a surface — which is why the checks are exact.

`circle` + `circumradius_for_width` correct for a tessellated circle being
*inscribed*, so a hole's flats reach the diameter actually named.

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. |
| `alloc` | | Required by `Polygon`, `triangulate`, `circle`. |
| `std` | | Implies `alloc`. |

## License

MIT

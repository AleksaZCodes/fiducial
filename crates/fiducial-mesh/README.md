# fiducial-mesh

**A sealed, printable case derived from the board that goes inside it.**

[![crates.io](https://img.shields.io/crates/v/fiducial-mesh.svg)](https://crates.io/crates/fiducial-mesh)
[![docs.rs](https://docs.rs/fiducial-mesh/badge.svg)](https://docs.rs/fiducial-mesh)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

`no_std` (+ `alloc`) generation of watertight triangle meshes from a board
outline, exported as binary **STL** (printable) or **GLB** (web viewer).

Nobody models the enclosure. The board declaration in
[`fiducial-eda`](../fiducial-eda) already states the outline, the process, the
connectors and where they sit — and that is enough to derive a gasket-sealed
two-part case with the openings already punched. Change the board width and the
case, the render and the spec sheet all move, without anyone remembering to.

## Install

```sh
cargo add fiducial-mesh --features std
```

## Use

```rust,ignore
use fiducial_geometry::BoardOutline;
use fiducial_mesh::{extrude_board, to_stl_binary, to_glb};

let outline = BoardOutline::new(100.0, 60.0);
let mesh    = extrude_board(&outline);

let stl = to_stl_binary(&mesh); // Vec<u8> — save as board.stl
let glb = to_glb(&mesh);        // Vec<u8> — save as board.glb
```

## What the case gives you

| Part | Detail |
|---|---|
| Base | grooved rim, cavity, optional standoff posts under the board |
| Lid | compression tongue, generated in print orientation |
| Gasket | seal ring — **print in TPU** |

Everything is derived rather than declared. `wall = 2×lip + groove`, so the seal
sets the wall thickness. Openings come from the connector families. Standoff
height lifts the rim rather than eating declared headroom.

**Fasteners are opt-in, and the reason is stated rather than hidden:** a 3 mm
screw on FDM widens the lip from 1.2 mm to 5.87 mm and the wall from 4.8 mm to
9.47 mm, taking a 100×60 board's case from 110×70 to 120×80. That cost should be
a decision, not a surprise.

## How it is verified

Not by eye. Every generated part is asserted **directed**-edge unique with
positive signed volume — which catches T-junctions and inverted solids that
simple edge-parity misses. Openings are then **ray-probed on the shipped STL
bytes**: each bore is confirmed void through its full depth and the lip beside it
confirmed solid.

An unfeatured case is byte-identical to one generated before cutouts and
fasteners existed, asserted on the STL bytes — so a new feature cannot silently
perturb existing output.

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. |
| `alloc` | | Required for mesh building. |
| `std` | | Implies `alloc`, enables the exporters. |

## License

MIT

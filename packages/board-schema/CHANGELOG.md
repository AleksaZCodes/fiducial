# @fiducial/board-schema

## 0.5.0

### Minor Changes

- ee3d5e9: Add `outline.enclosure.fastener_diameter_mm` to the board schema.
  
  Declaring a shaft diameter gives the generated case four corner screws that
  retain the lid — a clearance hole through the lid and a pilot hole in the base.
  Without them nothing clamps the lid, so the gasket only compresses while
  something external holds it shut.
  
  The screws pass through the outer lip, outboard of the gasket groove: a hole
  inside the gasket line would open the sealed cavity. That lip has to carry the
  hole with a printable wall either side, so declaring a fastener thickens the
  case wall — which is why it is opt-in rather than assumed.
  
  `parseBoardInterface()` rejects a non-positive diameter, mirroring
  `fiducial_eda::validate`.

## 0.4.0

### Minor Changes

- ee5399e: Add `connector.mount` and enclosure standoff options to the board schema.
  
  A connector may now declare where it meets the case wall, and `fid derive`
  punches an opening for it. `mount` carries `side` and `offset_mm` in board
  coordinates plus optional `width_mm` / `height_mm` / `z_offset_mm`; sizes
  default to the connector family's body envelope, newly exported as
  `CONNECTOR_OPENINGS` and resolved by `mountEnvelope(mount, type)`.
  
  `mount` is optional per connector, and its absence means no hole — the safe
  default in a sealed case, and the right answer for a header reached with the
  lid off.
  
  `outline.enclosure` gains `standoff_height_mm` and `standoff_size_mm`, which
  lift the board onto four corner posts.
  
  `parseBoardInterface()` validates all of it, mirroring `fiducial_eda::validate`:
  an unknown side, a negative offset, a non-positive size, a mount with no
  outline to measure against, and a connector family whose size cannot be
  resolved are all rejected.

## 0.3.0

### Minor Changes

- 10e652b: Add `outline.enclosure` to the `BoardInterface` schema — optional overrides for the generated case (`headroom_mm`, `lid_thickness_mm`, `gasket_width_mm`, `gasket_height_mm`, `gasket_compression`). `parseBoardInterface()` rejects non-positive values and a `gasket_compression` outside the exclusive range (0, 1), mirroring `fiducial_eda::validate`.
  
  These are the values a manufacturing process cannot imply. Clearance, lip width, floor thickness, and gasket fit remain derived from the board's tolerance class.

## 0.2.0

### Minor Changes

- 40b2124: Phase 14: `@fiducial/board-schema` — TypeScript types for `board.interface.json`, the machine-readable connector/pin/net interface between the EDA pipeline and the rest of the Fiducial platform.
  
  Also introduces:
  - `crates/fiducial-eda` — `no_std` Rust crate with `BoardInterface`, `Connector`, `Pin`, `PinDirection`, `NetClass` types; `validate()` function (std feature); 10 tests green.
  - `capabilities/eda` — `fid add eda` capability: installs `board/main.ato` (atopile source), `board/board.interface.json` (seed output), `pipelines/eda.toml` (fid derive config).
  - `fid derive` — new `fid-validate` executor validates JSON against declared schema in-process; no external tool required for CI.
  - CI `eda-pipeline` job — validates seed `board.interface.json` against the schema on every commit.
- 40b2124: Add `outline` to the `BoardInterface` schema — `width_mm`, `height_mm`, optional `thickness_mm` (default 1.6) and `tolerance` (`fdm` | `resin` | `cnc`). `parseBoardInterface()` now rejects non-positive dimensions and unknown tolerance classes, mirroring `fiducial_eda::validate`. When declared, `fid derive` generates the enclosure STL and GLB from this block.
- 40b2124: Phase 15: 3D geometry pipeline — `fiducial-geometry` + `fiducial-mesh` Rust crates with `no_std` geometry primitives, binary STL and glTF 2.0 GLB export; `@fiducial/viewer3d-react` React component loading GLB via Three.js + GLTFLoader + OrbitControls; FDM/Resin/CNC tolerance profiles.

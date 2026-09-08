# @fiducial/board-schema

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

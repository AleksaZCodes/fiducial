---
"@fiducial/board-schema": minor
---

Phase 14: `@fiducial/board-schema` — TypeScript types for `board.interface.json`, the machine-readable connector/pin/net interface between the EDA pipeline and the rest of the Fiducial platform.

Also introduces:
- `crates/fiducial-eda` — `no_std` Rust crate with `BoardInterface`, `Connector`, `Pin`, `PinDirection`, `NetClass` types; `validate()` function (std feature); 10 tests green.
- `capabilities/eda` — `fid add eda` capability: installs `board/main.ato` (atopile source), `board/board.interface.json` (seed output), `pipelines/eda.toml` (fid derive config).
- `fid derive` — new `fid-validate` executor validates JSON against declared schema in-process; no external tool required for CI.
- CI `eda-pipeline` job — validates seed `board.interface.json` against the schema on every commit.

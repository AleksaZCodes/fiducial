# Fiducial — Build Order & Current State

_Read this at the start of every session. Updated manually as phases complete._

---

## Current phase: Phase 16

**Phase 0 — complete ✅ (2026-09-06)**

| Deliverable | Status |
| --- | --- |
| Repo `AleksaZCodes/fiducial` (public, MIT) | ✅ https://github.com/AleksaZCodes/fiducial |
| `MISSION.md`, `STACK.md`, `LICENSE`, `IP-POLICY.md` | ✅ |
| `crates/fiducial` v0.1.0 → crates.io | ✅ published |
| `@fiducial/fiducial` v0.1.0 → js registry (`@fiducial` org created) | ✅ published |
| Design spec → `docs/specs/2026-09-06-fiducial-design.md` | ✅ |

**Phase 1 — complete ✅ (2026-09-06)**

> Monorepo skeleton: Turborepo, Changesets, host Cargo workspace, release CI.
> Done when a trivial package publishes and installs in a scratch project.

| Deliverable | Status |
| --- | --- |
| Turborepo (`turbo.json`) wiring `build`, `typecheck`, `lint`, `test`, `dev` | ✅ |
| `@changesets/cli` + `.changeset/config.json` | ✅ |
| `pnpm build` + `pnpm typecheck` green via turbo | ✅ |
| `cargo build` + `cargo test` green | ✅ |
| `.github/workflows/ci.yml` (Rust + JS, runs on PR + push to main) | ✅ |
| `.github/workflows/release.yml` (changesets/action → npm + crates.io) | ✅ |
| `CLAUDE.md` + `AGENTS.md` — portable project context for agents | ✅ |
| Secrets needed: `NPM_TOKEN`, `CARGO_REGISTRY_TOKEN` in repo settings | ⚠️ set manually |

**Phase 2 — complete ✅ (2026-09-06)**

> Vertical slice: `fiducial-core` (`no_std`) + `fiducial-wasm` + 4-target CI matrix.
> Done when one function runs in a browser, in Tauri, and blinks an LED on an RP2040.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-core/` — `#![no_std]`, `version()`, `DeviceId`, 6 tests | ✅ |
| `crates/fiducial-wasm/` — wasm-bindgen wrapper, `fiducialVersion()`, `DeviceId` JS class | ✅ |
| `firmware/` — separate Cargo workspace (Embassy 0.10, embassy-rp 0.10) | ✅ |
| `firmware/shared/` — target-agnostic helpers, blink timing constants | ✅ |
| `firmware/rp2040/` — Embassy blink importing fiducial-core via `fiducial-firmware-shared` | ✅ |
| 4-target `cargo check` (x86_64, wasm32, thumbv6m, thumbv7em) — all green locally | ✅ |
| CI: `spine` job matrix + `wasm-pack` job + `firmware` job added to `ci.yml` | ✅ |
| Flash to RP2040 + verify `defmt` log shows `fiducial-core` version | ⚠️ manual (no hw in CI) |

**Phase 3 — complete ✅ (2026-09-06)**

> `fid` CLI: `new`, `add`, `doctor`; `fiducial.lock`; guard configurable and shell-aware.
> Done when `fid new` yields a building repo, `fid doctor` is clean, and the §11 false-positive cannot recur.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-cli/` — `fid` binary, added to host workspace | ✅ |
| `fid new <name>` — scaffold: fiducial.toml, fiducial.lock, MISSION.md, AGENTS.md, .claude/settings.json, .gitignore, README.md, git init | ✅ |
| `fid add app\|module\|firmware <target>` — real capability install for next/tauri/worker/rp2040 | ✅ |
| `fid doctor` — checks config, lock, and template hash integrity | ✅ |
| `fid guard-check` — shell-aware PreToolUse hook; tokenizes before matching; §11 false-positive cannot recur | ✅ |
| `fiducial.lock` — SHA-256 per template file, records source version | ✅ |
| 11 guard unit tests covering false-positive cases, pipelines, sudo, env assignments | ✅ |
| `packages/cli/` — `@fiducial/cli` npm shim with 2 passing tests | ✅ |

**Phase 3b — complete ✅ (2026-09-06)**

> Capability mechanism: a capability installs, activates guard + skill.

| Deliverable | Status |
| --- | --- |
| `capability.rs` — `CapabilityDef` type, built-in registry, `install()`, `check_capability()` | ✅ |
| Built-in capabilities: `web-next`, `firmware-rp2040`, `tauri`, `worker-cloudflare` | ✅ |
| Each capability: template files + `SKILL.md` (mandatory per §3.3) + guard rules | ✅ |
| `fid capability list [--all]` — show installed + available | ✅ |
| `fid capability check [--capability <id>]` — conformance validation | ✅ |
| `fid capability new <name>` — scaffold a new first-party capability | ✅ |
| `fid add app next\|tauri\|worker` / `fid add firmware rp2040` — real installs | ✅ |
| `fid derive`, `fid upgrade`, `fid graph` — stubs with accurate help (Phase 4) | ✅ |
| Comprehensive `--help` on all commands with `long_about`, `after_long_help`, `long_help` | ✅ |
| `fid doctor` stays clean after capability install (lock re-baselines `fiducial.toml`) | ✅ |

**Phase 6 — complete ✅ (2026-09-07)**

> ROP characterization harness — golden-file tests for every pure function.
> Done when: Green on unmodified ROP; fails on injected change.
> Lives at: `/home/aleksa/dev/rop-harness/` (standalone repo — not in fiducial)

| Deliverable | Status |
| --- | --- |
| `rop/subject/game-logic.ts` — verbatim copy of ROP's pure game-logic (no I/O) | ✅ |
| `rop/subject/types.ts` — verbatim copy of ROP's shared types | ✅ |
| `rop/golden/game-logic.json` — 39 golden expected outputs across 10 exported functions | ✅ |
| `rop/game-logic.char.test.ts` — 36 characterization assertions; green on unmodified subject | ✅ |
| `rop/inject.char.test.ts` — 3 injection assertions; proves harness detects semantic divergence | ✅ |
| `vitest.config.ts` — isolated `forks` pool; each file gets its own worker | ✅ |

**Phase 5 — complete ✅ (2026-09-07)**

> Claude Code plugin: six-line activation block → guard active in any scratch repo.

| Deliverable | Status |
| --- | --- |
| `.claude-plugin/plugin.json` — plugin manifest (`name`, `version`, `commands`, `strict`) | ✅ |
| `hooks/hooks.json` — auto-discovered `PreToolUse` hook: `fid guard-check` on every Bash call | ✅ |
| `commands/platform.md` — `/fiducial:platform` skill: core commands, guard rules, propagation guide | ✅ |
| `templates/claude-settings.json.tmpl` — changed from direct hook config to the 6-line plugin activation block | ✅ |
| `templates/AGENTS.md.tmpl` — documents `/fiducial:platform` slash command, mentions plugin as guard source | ✅ |
| `fid doctor` — migration filter: only surfaces migrations for installed capabilities (no spurious warnings) | ✅ |
| End-to-end: `fid new scratch-test` → `.claude/settings.json` has 6-line block → `fid doctor: clean` | ✅ |

**Phase 9 — complete ✅ (2026-09-08)**

> `fiducial-quantity` + `fiducial-model` + `fid derive/graph` + types pipeline.
> Done when: A protocol edit regenerates TS types; stale artifact or violated assertion fails CI.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-quantity/` — `#![no_std]` `Quantity<D>`, `Tolerance`, `AssertionError`, 7 tests | ✅ |
| `crates/fiducial-model/` — `#![no_std]` `Fact<T>`, `Decision`, `PipelineMeta`, 4 tests | ✅ |
| `packages/wasm-bridge/` — `@fiducial/wasm-bridge` v0.1.0 — generated TS types entry point | ✅ |
| `packages/wasm-bridge/src/generated.ts` — generated by `cargo test --features ts -p fiducial-wasm` | ✅ |
| `crates/fiducial-wasm/` — `ts-rs` optional dep + `ts` feature; `ts_export` module exports boundary types | ✅ |
| `fid derive` — real impl: discovers `pipelines/*.toml` + built-in types pipeline; runs executors; records artifact hashes in `fiducial.lock [artifacts]` | ✅ |
| `fid derive --check` — re-hashes outputs vs lock, exits non-zero on stale artifacts | ✅ |
| `fid graph` — emits pipeline DAG in text / dot / json | ✅ |
| `fiducial.lock` — `[artifacts]` section with `ArtifactRecord` (pipeline, source_version, hash) | ✅ |
| CI: `types-pipeline` job — regenerates TS types, `git diff --exit-code` catches staleness | ✅ |
| CI: spine matrix extended to check `fiducial-quantity` and `fiducial-model` for all 4 targets | ✅ |

**Phase 11 — complete ✅ (2026-09-08)**

> `fiducial-protocol` (no_std framing) + `fiducial-tauri` (serial transport) + Tauri desktop capability.
> Done when the desktop app can open a USB serial port, frame a message, and decode a reply.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-protocol/` — `#![no_std]` frame encoder/decoder; `[MAGIC\|LEN_LO\|LEN_HI\|PAYLOAD\|CRC8]`; 9 tests | ✅ |
| `fiducial-protocol` — CRC-8 XOR fold; `FrameDecoder<N>` state machine; resync on bad magic; oversized frame rejection | ✅ |
| `crates/fiducial-tauri/` — `SerialTransport` (open, send, recv); `list_ports()`; `TransportError` | ✅ |
| `capabilities/tauri/` — 5-file template: `tauri.conf.json`, `Cargo.toml`, `build.rs`, `src/lib.rs`, `src/main.rs` | ✅ |
| Tauri commands: `cmd_list_ports`, `cmd_connect`, `cmd_disconnect`, `cmd_send`, `cmd_recv` | ✅ |
| `crates/fiducial-cli/src/capability.rs` — `tauri` capability expanded to 5 templates | ✅ |
| `crates/fiducial-cli/src/templates.rs` — 4 new `CAP_TAURI_*` constants + `raw()` entries | ✅ |
| CI: `fiducial-protocol` added to 4-target spine matrix; `apt-get install libudev-dev` for serialport | ✅ |
| `cargo clippy --all-features -D warnings` clean; `cargo fmt` applied; all 15 test suites green | ✅ |

**Phase 12 — complete ✅ (2026-09-08)**

> `firmware/rp2040` + `stm32`, probe-rs + defmt, LoRa via `lora-rs`.
> Done when `fid add firmware rp2040|stm32` yields a flashable project sharing L0 with the desktop app.

| Deliverable | Status |
| --- | --- |
| `firmware/stm32/` — STM32F401 Embassy firmware (Cortex-M4F, thumbv7em-none-eabihf); blinks PC13 via embassy-stm32 0.6 | ✅ |
| `firmware/Cargo.toml` — workspace extended to include `stm32` member + `embassy-stm32 0.6` dep | ✅ |
| `firmware/shared/` — `lora` feature added: `lora-modulation 0.1`, EU868 channel/DR constants | ✅ |
| Workspace-level `[profile.dev/release]` in `firmware/Cargo.toml`; per-crate tables removed | ✅ |
| `capabilities/firmware-rp2040/` — expanded from README-only to full flashable scaffold (10 templates) | ✅ |
| `capabilities/firmware-stm32/` — new capability: 9 templates + SKILL.md | ✅ |
| `fid add firmware stm32` — wired to `firmware-stm32` capability (was stub) | ✅ |
| CI: `firmware` job extended — `cargo check --target thumbv7em-none-eabihf` + `libudev-dev` install | ✅ |
| `cargo check --target thumbv6m-none-eabi` (rp2040) — green | ✅ |
| `cargo check --target thumbv7em-none-eabihf` (stm32) — green | ✅ |
| `cargo build --workspace` (host) — green | ✅ |

**Phase 14 — complete ✅ (2026-09-08)**

> EDA pipeline: atopile → KiCad → `board.interface.json` + fab outputs.
> Done when: A board change regenerates every output; `--check` catches staleness.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-eda/` — `#![no_std]` `BoardInterface`, `Connector`, `Pin`, `PinDirection`, `NetClass`, `Board` types; `serde` derive | ✅ |
| `fiducial-eda` — `validate(json: &str) -> Result<BoardInterface, ValidationError>` under `std` feature; 10 tests | ✅ |
| `capabilities/eda/` — `fid add eda`: installs `board/main.ato`, `board/board.interface.json` (seed), `pipelines/eda.toml` | ✅ |
| `pipelines/eda.toml` — `fid-validate` executor; `board/board.interface.json` tracked as artifact | ✅ |
| `fid derive` — `fid-validate` executor validates JSON schema in-process; no atopile/KiCad required for CI | ✅ |
| `fid add eda` — `AddTarget::Eda` variant; installs `eda` capability | ✅ |
| `packages/board-schema/` — `@fiducial/board-schema` v0.1.0 — TypeScript types + `parseBoardInterface()` | ✅ |
| CI `eda-pipeline` job — `cargo test --features std -p fiducial-eda` validates seed schema on every commit | ✅ |
| CI spine matrix extended to check `fiducial-eda` for all 4 targets | ✅ |
| `pnpm build` + `pnpm test` — all 15 workspace tasks green | ✅ |

**Phase 15 — complete ✅ (2026-09-08)**

> `fiducial-geometry` + `fiducial-mesh` + `viewer3d-react` + tolerance profiles.
> Done when: Board outline → generated enclosure → printable STL and a GLB on a marketing page.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-geometry/` — `#![no_std]` `Point2/3`, `Vec2/3`, `BoundingBox`, `Polygon`; tolerance profiles FDM/Resin/CNC; 11 tests | ✅ |
| `crates/fiducial-mesh/` — `#![no_std]` + alloc; `extrude_board()` → watertight box mesh (12 triangles, 24 vertices); 11 tests | ✅ |
| `to_stl_binary()` — binary STL (80-byte header + 50 bytes/triangle); correct header tag and size verified | ✅ |
| `to_glb()` — minimal glTF 2.0 binary; POSITION + NORMAL VEC3, SCALAR UNSIGNED_INT indices; GLB magic/version/length verified | ✅ |
| `packages/viewer3d-react/` — `@fiducial/viewer3d-react` v0.1.0 — `BoardViewer` React FC; Three.js + GLTFLoader + OrbitControls; auto-framing via Box3 | ✅ |
| 6 viewer3d tests: exports present, BoardViewer is function, GLB magic bytes, header offsets, STL size formula | ✅ |
| CI: `fiducial-geometry` + `fiducial-mesh` added to 4-target spine matrix | ✅ |
| CI: `mesh-gen` job — `cargo test -p fiducial-geometry` + `cargo test --features std -p fiducial-mesh` | ✅ |
| `cargo clippy --all-features -D warnings` clean; `cargo fmt` applied; all test suites green | ✅ |
| `pnpm build` + `pnpm typecheck` + `pnpm test` — all 17 workspace tasks green | ✅ |

**Phase 15 — enclosure generation (completing the done-condition)**

| Deliverable | Status |
| --- | --- |
| `generate_enclosure()` / `enclosure_for()` — watertight open-top tray (28 triangles: outer shell, cavity, mitred rim) | ✅ |
| `EnclosureParams::from_outline()` — clearance = 2× process XY accuracy, wall/floor = process min wall thickness; builder overrides | ✅ |
| Tolerance profiles now drive geometry — identical board yields a tighter enclosure on resin/CNC than FDM (asserted in tests) | ✅ |
| `Polygon` consumed — `BoardOutline::to_polygon()`; enclosure sizes from its bounding box, so a non-rectangular outline is a one-method change | ✅ |
| `outline` block added to `BoardInterface` (optional; `width_mm`, `height_mm`, `thickness_mm`, `tolerance`) — the declaration meshes derive from | ✅ |
| `validate()` rejects non-positive dimensions and unknown tolerance names; `ValidationError` implements `std::error::Error` | ✅ |
| `fid-mesh` executor in `fid derive` — writes `.stl`/`.glb` by extension, in-process, no CAD tool in CI | ✅ |
| `pipelines/enclosure.toml` shipped by `fid add eda`; outputs tracked in `fiducial.lock` | ✅ |
| Watertightness test — every undirected edge shared by exactly 2 triangles (both board and enclosure meshes) | ✅ |
| `crates/fiducial-cli/tests/enclosure_pipeline.rs` — 5 end-to-end tests: `fid new` → `add eda` → `derive` → STL/GLB validity, geometry tracks declaration, tolerance changes output, `--check` catches tampering, missing outline errors | ✅ |
| Marketing page — `apps/web/src/app/board/page.tsx` in `web-next` renders the generated GLB via `BoardViewer` | ✅ |
| `@fiducial/board-schema` mirrors `Outline` + validation; 5 new tests | ✅ |
| CI: `mesh-gen` job runs the end-to-end enclosure pipeline test | ✅ |

**Phase 15b — sealed case ✅ (2026-09-10)**

> Gasket-sealed two-part case, replacing the open tray as the default output.

| Deliverable | Status |
| --- | --- |
| `MeshBuilder` primitives — `cap` / `ring` / `band` — winding correct by construction | ✅ |
| `generate_case_base()` — grooved rim, 60 triangles; `generate_case_lid()` — compression tongue in print orientation, 44 triangles | ✅ |
| `generate_gasket()` — seal ring, 32 triangles; **TPU** documented at every surface (SKILL.md, pipeline toml, rustdoc, page copy) | ✅ |
| `CaseParams` — groove/tongue/wall all derived; `wall = 2×lip + groove`, so the seal sets wall thickness | ✅ |
| `Mesh::translated` / `mirrored_z` / `merge`; `case_exploded()` renders base + gasket + lid separated | ✅ |
| Watertightness (edge parity) asserted on all three parts, and preserved across mirroring | ✅ |
| Fit tests: lid footprint matches base, gasket narrower than its groove, tongue shallower than groove, compression math | ✅ |
| `outline.enclosure` — declares only what the process cannot imply (headroom, lid thickness, gasket cross-section, compression) | ✅ |
| Validation rejects non-positive overrides and compression outside (0, 1), NaN included | ✅ |
| `fid-mesh` addresses parts by stem, format by extension; unknown stem fails and lists valid names | ✅ |
| Web GLB copy is one commented line in `pipelines/enclosure.toml` — opt in, hash-tracked, no copy step | ✅ |
| 35 mesh tests, 16 eda tests, 10 CLI end-to-end tests, 21 board-schema tests | ✅ |

**Phase 15c — connector cutouts + standoffs ✅ (2026-09-10)**

> Openings punched from the connector declarations, and posts under the board.
> Closes two of the three gaps `docs/specs/2026-09-08-…` recorded against the case.
> Spec: `docs/specs/2026-09-10-connector-cutouts-and-standoffs.md`.

| Deliverable | Status |
| --- | --- |
| `MeshBuilder::punched_face` — mitred frame fanned from the outer corners around an inset grid, so a hole subdivides only the inside of a face and neighbouring surfaces need no change | ✅ |
| No-hole short-circuit — an unfeatured case is byte-identical to one generated before cutouts existed (asserted on the STL bytes) | ✅ |
| `Cutout` + `Case` API — validation separate from generation; `CaseError` names the connector and the field to change | ✅ |
| `tunnel()` — the prismatic void through a wall, subdivided by neighbouring holes so both ends meet their panels vertex for vertex | ✅ |
| `post()` — standoffs punched out of the cavity floor and grown upward, so base + posts stay one closed manifold | ✅ |
| `z_rim` includes standoff height — lifting the board lifts the rim, rather than eating declared headroom | ✅ |
| `Side` enum + `CONNECTOR_OPENINGS` table in `fiducial-geometry` — 9 families; `type` implies the body envelope | ✅ |
| `connector.mount` in `BoardInterface` (`side`, `offset_mm`, optional size and `z_offset_mm`); board coordinates, so an offset means the same on every edge | ✅ |
| `outline.enclosure.standoff_height_mm` / `standoff_size_mm`; absent means the board rests on the floor | ✅ |
| Declared size is the connector **body** — one process tolerance per side is added, so resin cuts a tighter opening than FDM | ✅ |
| Validation rejects: seal breach, off-wall, below-cavity, sub-min-feature, overlapping openings, oversized standoffs, unknown side/family | ✅ |
| Watertightness strengthened to **directed**-edge uniqueness + positive signed volume — catches T-junctions and inverted solids, which parity does not | ✅ |
| Ray-cast solid-membership tests — the openings are genuinely through-holes at the declared positions and the wall beside them is solid | ✅ |
| `fid derive` wires mounts into `Case` and validates before writing anything | ✅ |
| Seed board mounts USB-C + Qwiic and deliberately leaves SWD unmounted (8.5 mm would breach the seal); declares 3 mm standoffs | ✅ |
| `@fiducial/board-schema` mirrors `Mount`, `Side`, `CONNECTOR_OPENINGS`, `mountEnvelope()` and every validation rule | ✅ |
| Docs: SKILL.md (mount + family + rejection tables), pipeline toml, rustdoc, board-schema README | ✅ |
| 63 mesh tests, 30 eda tests, 18 geometry tests, 20 CLI end-to-end tests, 35 board-schema tests | ✅ |
| 4-target spine (x86_64 / wasm32 / thumbv6m / thumbv7em) green; clippy + fmt clean; STLs verified closed and oriented from the shipped bytes | ✅ |

**Phase 15d — fasteners + polygon triangulation ✅ (2026-09-11)**

> Four corner screws that actually retain the lid, and the primitive they needed.
> Closes the "lid is not retained" gap. Spec:
> `docs/specs/2026-09-11-fasteners-and-triangulation.md`.

| Deliverable | Status |
| --- | --- |
| `fiducial-geometry::triangulate` — ear clipping of a polygon with holes, with **exact** bridge visibility instead of earcut's ray-cast/tangent heuristic | ✅ |
| Bridges validated against **not-yet-merged** holes; a vertex already carrying a bridge cannot be a second target; an ear must have a valid **diagonal** — all three found by fuzzing, each silently dropped part of a surface | ✅ |
| Bridge duplicates get fresh indices for unambiguous topology, resolved back to input indices so callers only see the points they passed | ✅ |
| `circle()` + `circumradius_for_width()` — corrects for a polygon being inscribed, so a hole's *flats* reach the diameter named | ✅ |
| Verified by area conservation, per-triangle winding, and boundary-edge equality vs the input loops over 3000 fuzz cases + swept annulus rims and walls | ✅ |
| `MeshBuilder::flat_face` / `hole_wall` — a triangulated face with holes, and the bore through it | ✅ |
| Rim triangulated as one polygon when fastened: a corner screw sits exactly on the mitre of the four-trapezoid `ring()` and cannot be punched piecewise | ✅ |
| Clearance hole in the lid, 0.8× pilot in the base, both through the **outer lip** — outboard of the gasket, because a hole inside the gasket line opens the cavity | ✅ |
| `lip_outer_mm()` derived from the **tessellated bore's corner extent**, not the nominal diameter — sizing to nominal left 1.17 mm of wall against a 1.2 mm process minimum | ✅ |
| Wall cost stated, not hidden: a 3 mm screw on FDM widens the lip 1.2 → 5.87 mm and the wall 4.8 → 9.47 mm, so a 100×60 board's case goes 110×70 → 120×80. Opt-in for that reason | ✅ |
| `outline.enclosure.fastener_diameter_mm`; validation rejects a pilot below the process minimum feature | ✅ |
| An **unfastened** case is byte-identical to one generated before fasteners existed, asserted on the STL bytes | ✅ |
| Ray-probed on shipped STL bytes: 20 probes — every bore void through its full depth, lip beside it solid, in base and lid | ✅ |
| `@fiducial/board-schema` mirrors `fastener_diameter_mm` + validation; 37 tests | ✅ |
| 76 mesh, 34 geometry, 30 eda, 24 CLI end-to-end tests; 4-target spine, clippy, fmt clean | ✅ |

**Still open on the case** — fastener and standoff positions are derived rather
than declared, there is no countersink, openings are rectangular only (now a
small change rather than a missing capability, since `flat_face` takes any loop),
and the case carries no IP rating. `Polygon::signed_area()` still awaits
offsetting a non-rectangular outline — the one genuinely absent primitive.

**Phase 13 — complete ✅ (2026-09-08)**

> Web Serial, WebUSB, and BLE transports — same Fiducial frame codec as the firmware.
> Done when: Same device reachable from browser and phone with the same codec.

| Deliverable | Status |
| --- | --- |
| `packages/transport-web/` — `@fiducial/transport-web` v0.1.0 — browser transport package | ✅ |
| `src/codec.ts` — TypeScript port of `fiducial-protocol`: `crc8`, `encode`, `encodedLen`, `FrameDecoder` | ✅ |
| `src/transport.ts` — `Transport` interface (`AsyncIterable<Uint8Array>` + `send` + `close`) + `AsyncQueue<T>` | ✅ |
| `src/serial.ts` — `WebSerialTransport` (Web Serial API: Chrome/Edge 89+) | ✅ |
| `src/usb.ts` — `WebUsbTransport` (WebUSB API: Chrome/Edge 61+) | ✅ |
| `src/ble.ts` — `BleTransport` (Web Bluetooth API; Fiducial service/TX/RX UUIDs defined) | ✅ |
| `src/globals.d.ts` — minimal type stubs for experimental browser APIs not in TypeScript DOM lib | ✅ |
| 24 tests: codec roundtrip, CRC failure, noise resync, oversized rejection, AsyncQueue — all green | ✅ |
| `pnpm build` + `pnpm typecheck` — all 10 workspace packages green | ✅ |
| Codec is byte-exact with `fiducial-protocol` Rust crate — same MAGIC, same CRC-8 XOR fold, same frame layout | ✅ |

**Phase 10 — complete ✅ (2026-09-08)**

> Next.js + SvelteKit templates + thin L3 bindings.
> Done when: Both render the same tokens and the same headless logic.

| Deliverable | Status |
| --- | --- |
| `capabilities/web-next/` — updated templates: `@fiducial/tokens` + `@fiducial/headless` deps, `globals.css` with token CSS vars, `layout.tsx` imports globals, `page.tsx` uses `Result<T,E>` | ✅ |
| `capabilities/web-svelte/` — new capability: SvelteKit templates with `app.css` (same token vars), `+layout.svelte`, `+page.svelte` using `Result<T,E>` from headless | ✅ |
| `fid add app svelte` — wired to `web-svelte` capability (was stub) | ✅ |
| `packages/ui-react/` — `@fiducial/ui-react` v0.1.0 — `Button` + `StatusBadge<T,E>` built on token CSS vars + `@fiducial/headless` | ✅ |
| `packages/ui-svelte/` — `@fiducial/ui-svelte` v0.1.0 — `Button.svelte` + `StatusBadge.svelte` (same token vars, same headless types) | ✅ |
| `pnpm typecheck` — all 9 workspace packages green including ui-react and ui-svelte | ✅ |

**Phase 7 — complete ✅ (2026-09-07)**

> L1 tokens + L2 packages: each publishes and typechecks in isolation.

| Deliverable | Status |
| --- | --- |
| `packages/tokens/` — `@fiducial/tokens` v0.1.0 — typed color, spacing, typography, radii tokens | ✅ |
| `packages/tokens/src/tailwind.ts` — `fiducialPreset` for Tailwind CSS (no tailwindcss dep) | ✅ |
| `packages/headless/` — `@fiducial/headless` v0.1.0 — `Result<T,E>` + `OfflineQueue<T>` | ✅ |
| `pnpm build` + `pnpm typecheck` green across all 4 workspace packages | ✅ |
| Changeset: both packages at `minor` (0.1.0 initial release) | ✅ |

**Phase 4 — complete ✅ (2026-09-06)**

> Propagation: codemods + 3-way template merge.
> Done when an upstream template change and a renamed API both land in a scaffolded product, conflict surfaced correctly.

| Deliverable | Status |
| --- | --- |
| `lock.rs` — `base_content` field on `TemplateRecord`; `applied_migrations` list | ✅ |
| `templates.rs` — central registry of current in-binary template content by path | ✅ |
| `migration.rs` — codemod migration structs, built-in registry, `apply_pending`, `pending` | ✅ |
| Built-in migrations: `web-next/0.2.0/font-inter-to-geist`, `worker-cloudflare/0.2.0/wrangler-compatibility-date` | ✅ |
| `fid upgrade` — 3-way template merge (base from lock, ours from disk, theirs from binary) using `diffy` | ✅ |
| `fid upgrade` — codemod application in version order, recorded in `fiducial.lock` | ✅ |
| `fid upgrade` — skill file refresh (platform-owned, always overwrites) | ✅ |
| `fid upgrade --dry-run` — reports changes without writing | ✅ |
| `fid doctor` — reports templates with upstream changes available | ✅ |
| `fid doctor` — reports pending codemod migrations | ✅ |
| 10 new tests: `lock::tests` (base_content, 3-way merge), `migration::tests` (apply, dry-run, idempotence) | ✅ |

---

## Repo layout (current)

```
fiducial/
├── MISSION.md
├── STACK.md
├── CLAUDE.md           agent context (Claude Code)
├── AGENTS.md           agent context (Codex, Copilot Workspace, etc.)
├── LICENSE             MIT
├── IP-POLICY.md
├── README.md
├── PHASES.md           ← this file
├── turbo.json          Turborepo pipeline
├── Cargo.toml          host workspace
├── package.json        pnpm workspace root (private)
├── pnpm-workspace.yaml
├── .changeset/         Changesets config + pending changesets
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml      Rust + JS CI (PR + main)
│       └── release.yml Changesets release automation
├── crates/
│   ├── fiducial/            crates.io placeholder
│   ├── fiducial-core/       no_std spine — version(), DeviceId
│   ├── fiducial-quantity/   no_std Quantity<D>, Tolerance, AssertionError
│   ├── fiducial-model/      no_std Fact<T>, Decision, PipelineMeta
│   ├── fiducial-protocol/   no_std frame codec — MAGIC/LEN/PAYLOAD/CRC8
│   ├── fiducial-eda/        no_std BoardInterface schema + validate()
│   ├── fiducial-geometry/   no_std 2D/3D primitives + tolerance profiles
│   ├── fiducial-mesh/       no_std extrusion, enclosure, STL + GLB export
│   ├── fiducial-tauri/      SerialTransport for the desktop app
│   ├── fiducial-wasm/       wasm-bindgen bindings → fiducial-core
│   └── fiducial-cli/        `fid` binary — new, add, derive, doctor, guard-check
├── firmware/                SEPARATE Cargo workspace (excluded from root)
│   ├── Cargo.toml           workspace root
│   ├── rust-toolchain.toml  stable + embedded targets
│   ├── shared/              target-agnostic helpers (blink constants, LoRa)
│   ├── rp2040/              Embassy blink — thumbv6m-none-eabi
│   └── stm32/               Embassy blink — thumbv7em-none-eabihf
├── packages/
│   ├── fiducial/            @fiducial/fiducial npm placeholder
│   ├── cli/                 @fiducial/cli npm shim for `fid`
│   ├── tokens/              @fiducial/tokens — L1 design tokens + theme CSS
│   ├── headless/            @fiducial/headless — L2 Result<T,E> + OfflineQueue<T>
│   ├── ui-react/            @fiducial/ui-react — shadcn/Base UI registry
│   ├── ui-svelte/           @fiducial/ui-svelte — same tokens, Svelte
│   ├── wasm-bridge/         @fiducial/wasm-bridge — generated TS types
│   ├── transport-web/       @fiducial/transport-web — Web Serial/USB/BLE
│   ├── board-schema/        @fiducial/board-schema — board.interface.json types
│   └── viewer3d-react/      @fiducial/viewer3d-react — GLB viewer (Three.js)
└── docs/
    └── specs/
        ├── 2026-09-06-fiducial-design.md
        └── 2026-09-08-phases-13-15-transports-eda-geometry.md
```

---

## Source material (local, not in this repo)

- `/home/aleksa/dev/rop-reference/` — ROP snapshot at b16b226; characterization baseline for Phase 6. Never modify, never push (contains real secrets).
- `/home/aleksa/dev/rop-harness/` — standalone characterization harness (Phase 6). Its own git repo; not part of the fiducial workspace.
- `/home/aleksa/dev/platform/` — retired planning folder. Superseded by this repo.

---

## Phase build order (spec §16)

| Phase | Deliverable | Done when | Status |
| --- | --- | --- | --- |
| **0** | Claim names; repo; MISSION, LICENSE, IP-POLICY | Both publish --dry-run pass | ✅ |
| **1** | Monorepo skeleton: Turborepo, Changesets, CI | Trivial package publishes and installs in scratch project | ✅ |
| **2** | Vertical slice: `fiducial-core` no_std + WASM + 4-target CI | One fn runs in browser, Tauri, and blinks LED on RP2040 | ✅ |
| **3** | `fid` CLI: `new`, `add`, `doctor`; shell-aware guard | `fid new` builds; `fid doctor` clean; false-positive cannot recur | ✅ |
| **3b** | Capability mechanism | A capability installs, activates guard + skill | ✅ |
| **4** | Propagation: codemods + 3-way template merge | Upstream change lands in product, conflict surfaced correctly | ✅ |
| **5** | Claude Code plugin | Six-line block → guard active in scratch repo | ✅ |
| **6** | ROP characterization harness | Green on unmodified ROP; fails on injected change | ✅ |
| **7** | L1 tokens + L2 packages | Each publishes and typechecks in isolation | ✅ |
| **8** | ROP migration wave 1 (optional, on your schedule) | Every PR green on all tiers + characterization | ⬜ |
| **9** | `fiducial-quantity` + `fiducial-model` + `fid derive/graph` + types pipeline | A protocol edit regenerates TS types; stale artifact or violated assertion fails CI | ✅ |
| **10** | Next.js + SvelteKit templates + thin L3 bindings | Both render the same tokens and the same headless logic | ✅ |
| **11** | Tauri desktop + `fiducial-tauri` + `fiducial-protocol` | Desktop app talks to a device over USB serial | ✅ |
| **12** | `firmware/rp2040` + `stm32`, probe-rs + defmt, LoRa via `lora-rs` | `fid add firmware` yields a flashable project sharing L0 with the desktop app | ✅ |
| **13** | Web Serial/WebUSB + BLE transports | Same device reachable from browser and phone with the same codec | ✅ |
| **14** | EDA pipeline: atopile → KiCad → `board.interface.json` + fab outputs | A board change regenerates every output; `--check` catches staleness | ✅ |
| **15** | `fiducial-geometry` + `fiducial-mesh` + `viewer3d-*` + tolerance profiles | Board outline → generated enclosure → printable STL and a GLB on a marketing page | ✅ |
| **16** | Workbench v0: `fid dash` read-only view | Roadmap, status, decisions, CI, graph, freshness in one place | ⬜ |
| **16b** | `fid release` + version-skew assertions | A protocol bump fails any artifact still on the old version; compatibility matrix committed | ⬜ |
| **16c** | Firmware OTA: `embassy-boot` A/B, signing, resumable transfer, staged rollout | Device updates over BLE, self-tests, marks booted — broken image rolls back automatically | ⬜ |
| **17** | `fiducial-sim`, `realtime`, workbench v1 | Simulation runs native and in WASM; realtime's three contracts covered by tests | ⬜ |
| **18** | ROP migration wave 2 (optional) — eligible rules to L0 Rust | Each differential-tested before the TypeScript is deleted | ⬜ |

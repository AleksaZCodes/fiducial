# Fiducial — What Shipped

_The record of what was **built**, phase by phase. For what is **intended**, see
[`ROADMAP.md`](ROADMAP.md)._

_Formerly `PHASES.md`. Renamed 2026-09-14 — the name read as a plan, which is the
roadmap's job. See `docs/specs/2026-09-14-phases-renamed-to-shipped.md`. Phase
numbering is unchanged._

---

## Current phase: between phases — next is external capabilities

_Roadmap items 1 (i18n) and 2 (capability taxonomy) are complete. Item 3,
external capabilities, is the keystone: every capability built before it exists
is one more compiled into the binary that later has to be migrated out._

> Build order beyond this phase: [`ROADMAP.md`](ROADMAP.md).

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
than declared, there is no countersink, openings are rectangular only, and the
case carries no IP rating. Round openings are further off than they look:
`flat_face` and `hole_wall` are both XY-plane-only, while a connector opening is
a vertical face with a horizontal bore, so it needs those primitives
parameterised the way `punched_face` already is. `Polygon::signed_area()` still awaits
offsetting a non-rectangular outline — the one genuinely absent primitive.

**Phase 16 — Workbench v0 ✅ (2026-09-11)**

> `fid dash` — one read-only view of roadmap, status, decisions, CI, graph, freshness.
> Spec: `docs/specs/2026-09-11-workbench-v0-fid-dash.md`.

| Deliverable | Status |
| --- | --- |
| `fid dash` — seven sections: product, git, roadmap, decisions, CI, graph, freshness | ✅ |
| **Owns no store.** Every number recomputed per run from files already in the repo; a test asserts dash writes nothing at all | ✅ |
| **No network.** CI read from declared workflow files, not a live API — a test renders it with every proxy pointed at a closed port | ✅ |
| Reports whether *any* workflow runs `fid derive --check`; a fresh scaffold has none, so a stale artifact would reach main — dash says so | ✅ |
| Absence is a finding: no roadmap / no decisions / no pipelines each render a "not declared" line and still exit 0 | ✅ |
| Exits 0 even on findings — gating is `fid derive --check` and `fid doctor`; a dashboard must be runnable casually | ✅ |
| Freshness re-hashes artifacts (fresh / stale / missing / **never derived**) and reports hand-edited templates as drift | ✅ |
| Roadmap counted from `✅ 🟡 ⬜` and task-list markers in `ROADMAP.md` or `SHIPPED.md`; unmarked prose counts as nothing | ✅ |
| Decisions read from `docs/specs` (or `docs/decisions`, `docs/adr`), newest first by date-prefixed filename, titled from the first heading | ✅ |
| Git handles the unborn HEAD a fresh `fid new` leaves — `is_repo` is tracked separately, because inferring it from a missing branch called every new product un-versioned | ✅ |
| `--json` — same facts for agents and workbench v1, so neither reimplements these reads | ✅ |
| `--section <name>`; an unknown name lists the valid ones | ✅ |
| **`pipeline::discover` — one reader for `pipelines/*.toml`.** `derive` and `graph` each parsed it and *disagreed*: derive failed with the filename, graph labelled it `"unknown"`. Dash would have been the third copy | ✅ |
| Scaffolded `AGENTS.md` + `README.md` templates now point agents at `fid dash` / `--json` | ✅ |
| `fid graph` help no longer claims "not yet implemented (Phase 4)" | ✅ |
| CI: `workbench` job — pipeline-discovery unit tests + 21 end-to-end dash tests | ✅ |
| 21 dash end-to-end, 7 pipeline unit, 24 enclosure, 34 CLI lib tests; clippy + fmt clean | ✅ |

**Phase 16 review (2026-09-11)** — a pass over what had just shipped, decided
against `MISSION.md`, found six defects:

| Found | Was | Mission line it failed |
| --- | --- | --- |
| Non-ASCII decision filename | **Panicked** — byte-sliced a filename at 10 | "indistinguishable in quality from a funded team's" |
| Em-dash in a long roadmap row | **Panicked** — `String::truncate` mid-character | same |
| Commented-out `derive --check` | Reported the repo as **guarded when it was not** — inverting dash's most useful finding | "legible to a machine… safe rather than hopeful" |
| A job named `push` | Reported as a **trigger** of a manual-only workflow | same |
| A roadmap legend line | Counted as **one completed item** | same |
| `pipeline::discover` called twice in one function | Two reads of one declaration | principle 1 |
| ANSI escapes in redirected output | Escape codes written into pipes and logs | quality bar |

Also swept the CLI's own help text, which had drifted the same way `CLAUDE.md`
had: `fid derive` — the central command of the system — advertised itself as
"Not yet implemented (Phase 4)" and told readers to run `cargo build` instead;
`svelte` and `stm32` were labelled "(Phase 3b)" long after they worked; `mobile`
and `module` promised a phase that had already shipped. **Help text now names no
phase at all** — a schedule is a fact `SHIPPED.md` owns, and a second copy of it
drifts. A `help_honesty` test enforces that, and CI runs it.

**Still open on the workbench** — single repo only (multi-repo is v1 per §10),
no live CI status (deliberate; would go behind a flag), decisions are listed but
not read so supersession is undetected, and `briefs` from §10's v0 row is not
built because no product has one yet.

**Phase 22 — i18n: localized by construction ✅ (2026-09-14)**

> Principle **1c** made mechanical. Spec: `ROADMAP.md` § i18n.

| Deliverable | Status |
| --- | --- |
| `MISSION.md` **1c** — a user-visible string is a fact; every locale a derivation that must exist. Generated into every scaffolded product's `AGENTS.md` | ✅ |
| `@fiducial/i18n` — locale negotiation, strict lookup, money, dates, timezones, plurals, relative time. Native `Intl` only, so it runs on server, client and edge | ✅ |
| **Money is a fact that carries its unit** — integer minor units, currency independent of locale, cross-currency arithmetic refuses, `allocate()` loses nothing. `JPY` has 0 minor digits and `KWD` has 3 | ✅ |
| Timezone inverse solved by **offset probing**, not arithmetic — a zone's offset depends on the instant, which is circular across a DST boundary. Verified through both EU transitions | ✅ |
| `fid add i18n` — JSON catalogs, pipeline, and a `SKILL.md` at a **vendor-neutral path** | ✅ |
| `fid-i18n` executor — catalogs in, a typed `MessageKey` union out. A mistyped key is a compile error | ✅ |
| **The gate:** a missing translation fails `fid derive --check`, naming the file and the key | ✅ |
| **Reference is the union of all keys, not the default locale's.** Hand-testing found that using the default as reference let a key deleted *from the default* pass as another locale's "extra" — the ROP failure with the locales swapped | ✅ |
| Placeholder mismatch fatal; reordered placeholders accepted, because languages order words differently | ✅ |
| Untranslated-looking values reported, never fatal — failing there would block a legitimate `"Wi-Fi"` | ✅ |
| Installing a capability **seeds the declaration it needs**; a pipeline with an empty declaration fails on the next derive | ✅ |
| 43 runtime tests, 10 comparison unit tests, 10 end-to-end. 395 across the workspace | ✅ |

**Phase 23 — capability taxonomy, made real ✅ (2026-09-15)**

> Declarations, pipelines and adapters become first-class in the CLI.
> Spec: `docs/specs/2026-09-15-capability-taxonomy-made-real.md`, extending the
> accepted `2026-09-14-capability-taxonomy.md`.

| Deliverable | Status |
| --- | --- |
| `CapabilityDef` gains `declarations`, `pipelines`, `requires_adapters` — a capability can now **say what it contributes**, by kind | ✅ |
| **The `if cap.id == "i18n"` branch is gone.** `patch_config` had to know that the i18n capability introduces an `[i18n]` block, because the capability had no way to say so; the next one with a config block would have added a second branch | ✅ |
| `Declaration::ConfigBlock { name, seed }` carries a `fn(&mut Config)`, so the capability that owns a fact owns filling it in | ✅ |
| `eda` and `i18n` migrated: the board interface and the message catalogs are declarations, the four `pipelines/*.toml` are pipelines, `main.ato` stays a template. Generalized from two working cases, not designed | ✅ |
| Declarations install **before** pipelines — a pipeline whose declaration is not yet on disk fails on the next derive, and install order is the cheapest place to make that impossible | ✅ |
| `[adapters]` in `fiducial.toml` — one vendor per contract, as a map rather than a struct, because a struct would be a second copy of the contract list | ✅ |
| Five contracts: `database`, `storage`, `deploy`, `email`, `errors` | ✅ |
| **Every contract implements exactly `none` today, and nothing else.** A selectable vendor is a promise — and this repository had just spent a phase removing five guard-rule names that were declared, counted by `fid dash`, and enforced by nothing. An adapter registry offering `supabase` before anything speaks Supabase is that bug with a different noun | ✅ |
| `candidates` records where each contract is heading, kept out of `implementations` so nothing can select one and believe it works | ✅ |
| A planned vendor and a typo get **different messages** — `supabase` is a "not yet", `supabse` is a "no". One "unknown value" message answers both the same way | ✅ |
| `none` is a real no-op, not a placeholder: diagnostics wired in from the first commit, costing nothing until pointed somewhere | ✅ |
| A capability may **require** a contract without choosing the vendor; `fid doctor` fails when nobody filled it | ✅ |
| `check_capability` rejects a pipeline outside `pipelines/` (installed and never run), a `pipelines/` file filed as a template (installed and never gated), a capability with pipelines and no declarations, and an unknown required contract | ✅ |
| `every_builtin_capability_conforms` runs those against the registry itself — a first-party capability cannot ship violating a rule the CLI enforces on everyone else's. Verified by breaking a real capability | ✅ |
| `fid dash --section taxonomy` — every declaration with whether it is **present**, every contract with what satisfies it, including the ones nothing selected. An `[adapters]` key that is not a contract is appended rather than dropped | ✅ |
| `fid capability list` says what each capability declares, derives and requires; `--all` prints the contracts, because the set lives in the binary and nobody can select what they have not been told exists | ✅ |
| `fid capability new` teaches the four kinds and the test for a declaration | ✅ |
| 5 capability invariants, 6 adapter unit tests, 9 end-to-end. Clippy (0 warnings), fmt, full Rust suite and 17 JS tasks green | ✅ |
| **Deliberately not done:** no real adapter implementations (roadmap item 6 — designing contracts against no consumer is the failure the spec names); no adapter *capability*, because a contract is a slot a product fills, not something you `fid add` | ✅ |

**Phase 22b — i18n, closed out ✅ (2026-09-14)**

> The two items Phase 22 left open. Nothing on i18n remains.

| Deliverable | Status |
| --- | --- |
| **Hardcoded-string detector** — user-visible literals that never reached a catalog, found in `.tsx` / `.jsx` / `.svelte` / `.vue` / `.astro` | ✅ |
| **Warns, never fails.** The detector cannot tell prose from a `data-testid` with certainty; making it fatal means every false positive blocks someone until the rule is loosened for everyone | ✅ |
| **Reported, not logged** — a count and a file:line list in `fid doctor`, a `Localization` section in `fid dash`, and the same facts under `--json` so an agent consumes them as data | ✅ |
| Kept out of `doctor`'s `warnings`, whose summary line calls them "upgrade hint(s)" — a hardcoded string is not something `fid upgrade` fixes | ✅ |
| Nine false-positive rules, each of which earned its place: `className`, `data-testid`, `role`, URLs, paths, MIME types, dotted keys, `{t("…")}` call sites, `{count} items` | ✅ |
| `>text</` required, not `>` … `<` — the first version read `if (a > b && c < d)` as a tag pair | ✅ |
| `aria-label` no longer also fires as `label`; attribute names must match whole | ✅ |
| **`i18n-ignore` now does something.** `SKILL.md` had documented the escape hatch since the capability shipped, and nothing honoured it | ✅ |
| A product declaring no locales is never nagged — noise is the one thing a warning cannot survive | ✅ |
| **`fid new --locales`** — default `sr,en`; `--locales none` opts out for a CLI or firmware image | ✅ |
| `--default-locale` is **declared, never taken from list order**; `--locales en,fr` without it fails and names the fix | ✅ |
| A declared locale with no shipped catalog is seeded from the default, so the gap is *reported as untranslated* rather than silently absent; a catalog for an undeclared locale is removed, because the executor reads the locale set from the directory listing | ✅ |
| `DEFAULT_LOCALES` / `DEFAULT_LOCALE` declared once and shared by `fid new` and the capability's seeding | ✅ |
| **`fid new` leaves the scaffold derived** — the CI it also scaffolds runs `fid derive --check`, so a product used to fail its own pipeline on the first commit, before anyone had changed anything | ✅ |
| `fid new` re-records `fiducial.toml` after patching it, or the product is born reporting its own config as hand-edited | ✅ |
| 10 detector unit tests, 8 end-to-end; 79 CLI lib tests. Clippy, fmt, full Rust suite and 17 JS tasks green | ✅ |

**Phase 21b — namespaced agents ✅ (2026-09-14)**

> A subagent's filename is its identity, and `design` / `review` are names other
> people use too.

| Deliverable | Status |
| --- | --- |
| Scaffolded agents renamed `design` → `fiducial-design`, `review` → `fiducial-review` (file name **and** frontmatter `name:`) — a colliding agent is silently unavailable, not an error | ✅ |
| `templates::RENAMED_TEMPLATES` — the declaration of what moved. A rename **cannot** be a codemod: `MigrationOp` is a literal search-and-replace within one file, and this is a file moving | ✅ |
| `fid upgrade` installs the new path and removes the old one — ordered **before** the "added" pass, which would otherwise install the new file and leave the colliding one behind | ✅ |
| A **locally modified** file at a renamed path is never deleted: both are kept, with a warning naming the fix. Discarding someone's edits to solve a naming problem is the worse outcome | ✅ |
| Propagated to the real `fon` product and verified clean | ✅ |
| 2 new end-to-end tests; captures regenerated so the guides show the new names | ✅ |

**Phase 21 — documentation that cannot lie ✅ (2026-09-14)**

> Guides for humans and agents, with terminal output generated from the real
> binary and gated in CI. Spec: `docs/specs/2026-09-14-phase-21-documentation.md`.

| Deliverable | Status |
| --- | --- |
| `docs/guides/start-here.md` — the paradigm, opening with a concrete failure (one connector written down six times, and the fab run that finds out) rather than the thesis | ✅ |
| `docs/guides/first-product.md` — nothing → board → generated enclosure → CI gate in ~20 min; has the reader **break** `fid derive --check` deliberately, because the guarantee is only believable once seen failing | ✅ |
| `docs/guides/for-agents.md` — orientation, the five that bite, the finish checklist | ✅ |
| `docs/guides/README.md` — index, and why the terminal output is generated | ✅ |
| **Captures generated from the real binary**, same freshness gate as `docs/protocol/vectors.json`; verified adversarially by tampering with one character | ✅ |
| Captures run **in walkthrough order against one product** — the first harness captured `fid graph` on a bare scaffold and embedded "no pipelines declared" into a section that comes *after* they are installed: real output, faithfully generated, completely misleading | ✅ |
| Captures **embedded** in the guides, not linked — GitHub renders no transclusion, and output behind a link nobody clicks shows nothing | ✅ |
| Both tests derive from `render()` rather than one reading files the other writes — the first version had an undeclared ordering dependency, and cargo runs tests in parallel | ✅ |
| Orphan detection: every declared capture is shown in some guide | ✅ |
| CI `docs` job; root README opens with a path in for new readers | ✅ |
| All internal markdown links verified to resolve | ✅ |
| **Deliberately not done:** no docs site (a second place for docs to drift), no Playwright screenshots (heavy dependency for a CLI-first platform) | ✅ |

**Phase 20 — Ring of Pursuit catalogued ✅ (2026-09-14)**

> `fid harvest` turned on its intended donor. Catalogue:
> `docs/harvest/ring-of-pursuit.md`.

| Deliverable | Status |
| --- | --- |
| Full catalogue of ROP's reusable assets — 8 entries, each with a target, a recipe and the warnings someone would otherwise rediscover | ✅ |
| An explicit **not worth lifting** list with reasons — as valuable as the first, and shorter to act on | ✅ |
| **Ratio: 1 extracted, 9 catalogued.** MISSION's two anti-goals pull against each other here; importing everything reusable would satisfy "do not rebuild" and violate "the platform must never become the project" | ✅ |
| **Extracted:** `OfflineQueue` durability — the one item that had already met the second-use bar, because `@fiducial/headless` had declared it needed this queue and shipped half of it | ✅ |
| `QueueStorage<T>` adapter (memory + durable), `DEFAULT_BACKOFF_SCHEDULE_MS`, `backoffFor()`, `pending()`; `maxRetries` now *derives* from the schedule instead of being a second declaration of it | ✅ |
| 7 new headless tests incl. surviving a reload, and one asserting the durable and in-memory paths run identical assertions so they cannot drift | ✅ |
| Changeset recording the `maxRetries` default change rather than letting it land silently | ✅ |
| **Defect in `fid harvest` found by dogfooding:** `.vitepress/cache`, `.wrangler/tmp`, `supabase/.temp` were walked, ranking a 13,206-line VitePress dependency chunk as the donor's most valuable business logic. 13 scratch directories added to the skip list; logic 61,560 → 16,887 real lines, contract 142,764 → 9,514 | ✅ |

**Phase 19 — harvest ✅ (2026-09-14)**

> Getting the good parts out of a codebase you already built, without dragging
> the rest along. Spec: `docs/specs/2026-09-14-phase-19-harvest.md`.

| Deliverable | Status |
| --- | --- |
| `fid harvest <path>` — walks a donor, classifies every file, detects its stack, stages readable copies | ✅ |
| **Eight kinds** — `logic`, `ui`, `theme`, `art`, `principle`, `ops`, `contract`, `test` — the four the request named plus the three that travel with them | ✅ |
| Every classification records the **evidence** for itself; the survey prints it, so a wrong guess is correctable rather than authoritative | ✅ |
| **The staging rule: `harvest/` is never the product.** Nothing wired in, nothing overwritten — asserted by fingerprinting every file outside `harvest/` before and after | ✅ |
| Survey ordered by **value per unit of risk**, not size: theme → principle → contract → logic → ui → ops → art | ✅ |
| `.env`, lockfiles and logs never inventoried **or** staged; a secret planted in a test donor is asserted absent from the whole staging tree | ✅ |
| `node_modules`, `dist`, `target`, `.git` + 18 more never walked | ✅ |
| Harvesting a tree into itself refused — the first implementation `canonicalize()`d a path that does not exist yet, so the guard was skipped exactly when needed | ✅ |
| Markup classified as UI — without it the command was useless for the commonest donor there is, a static site | ✅ |
| `/fiducial:harvest` skill — the judgment half; presents *what is not worth lifting and why* as prominently as what is, then waits for a decision | ✅ |
| `docs/guides/harvesting.md` — the guide, with a worked landing-page example | ✅ |
| 10 end-to-end tests + 9 classifier unit tests | ✅ |

**Phase 18 — platform audit + cleanup ✅ (2026-09-14)**

> A pass over the whole platform asking not "what is broken?" but "where does
> this repository violate the rule it exists to enforce?" — because everything
> was already green: all tests passing, clippy clean, no TODO in the tree.
> Spec: `docs/specs/2026-09-14-phase-18-platform-audit.md`.

| Found | Was | Resolved |
| --- | --- | --- |
| No `[workspace.dependencies]` | `serde` declared **5×**, `serde_json` 4×, `sha2` 3× — principle 1 violated in the repo that states principle 1 | One declaration, inherited with `{ workspace = true }`; feature sets stay per-crate, deliberately |
| TypeScript version | Declared in 10 `package.json` files as **three different answers** (`^7.0.2`, `^7.0.0`, `^5.0.0`) with one version installed — drift, already arrived | pnpm `catalog:`; every package references it |
| `fid new` scaffolded no CI | Every product **born carrying the defect `fid dash` reports** — and a test *asserted* the scaffold had no freshness guard, so the bug was the spec | Scaffolds `ci.yml` running `fid doctor` + `fid derive --check`; test asserts the fix, keeps the negative case |
| `git init` without `--initial-branch` | Products born on `master` while the guard rule (`no-direct-main-push`), the review agent (`git diff main...HEAD`) and CI all named `main` | Forces `main`, `symbolic-ref` fallback for git < 2.28 |
| `ui-svelte` typecheck | An **`echo`** — and its tsconfig excluded the only file it would have checked. Unchecked for two phases while `pnpm typecheck` reported green | Real `svelte-check`; first run found an a11y defect in `Dialog.svelte` whose handler was also dead code |
| CI spine matrix | **Nine copies of one list**; `fiducial-sim` was already missing a tenth block | List **derived** from `#![no_std]` — the attribute is the declaration, the grep is the derivation |
| `fiducial-sim` "in spine matrix" | SHIPPED.md claimed it; it never was, and **cannot be** (uses `Vec` + rayon, not `no_std`) | Record corrected rather than quietly satisfied; sim checked on host + `wasm32` |
| 13 crates, 0 READMEs | Every published crate had a bare crates.io page | 17 READMEs written; each crate's is **run as a doctest**, which immediately caught a wrong method name |
| `CLAUDE.md` / `AGENTS.md` trees | Both stale (missing ota, sim, realtime; AGENTS miscounted 11/10 vs 13/11) and both carrying a disclaimer to run `ls` instead | Corrected; a disclaimer is not a fix, so a test now fails the build on a missing entry |
| pnpm workspace globs | `apps/*`, `workbench`, `cli` — none had ever existed; a stale glob is silent | Removed; a test asserts every glob resolves |

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-cli/tests/workspace_hygiene.rs` — **7 invariants**, each failing with the exact file and line to change | ✅ |
| `[workspace.dependencies]` — no crate manifest contains a version literal, asserted | ✅ |
| `catalog:` in `pnpm-workspace.yaml` — no shared JS dependency declared twice, asserted | ✅ |
| 13 crate READMEs + 4 package READMEs; every crate README compiled as a doctest | ✅ |
| `templates/ci.yml.tmpl` — scaffolded CI that gates on artifact freshness | ✅ |
| CI `spine` job derives its crate list from `#![no_std]` | ✅ |
| CI host job gains `--all-features` so README doctests are not silently skipped | ✅ |
| Root `README.md` rewritten from a 20-line stub into a real front page | ✅ |
| `fon` scaffold brought in line with the corrected templates | ✅ |
| **No behaviour, API or generated output changed** — geometry, wire format, OTA and dash all byte-identical | ✅ |
| 4-target spine, clippy, fmt clean; full Rust suite + 30 JS tasks green | ✅ |

**Phase 17 — complete ✅ (2026-09-10)**

> `fiducial-sim`, `realtime`, workbench v1.
> Done when: simulation runs native and in WASM; realtime's three contracts covered by tests.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-sim/` — `std` + `parallel` (rayon) on host; `no-default-features` on `wasm32` | ✅ |
| `ode::OdeSystem<N>` — one trait declaration; all simulations implement it | ✅ |
| `rk4_step` + `Integrator<S, N>` — 4th-order Runge-Kutta, generic over system and state size | ✅ |
| `Integrator::run_batch` — parallel with rayon on host, sequential on WASM; API identical | ✅ |
| `ThermalModel` — single-node RC thermal sim; `steady_state()` and `simulate()` | ✅ |
| `ThermalSnapshot` — serde round-trips; 7 host tests + 1 doc-test | ✅ |
| `packages/realtime/` — `@fiducial/realtime` TS package | ✅ |
| Contract 1: **Broadcast** — typed event map; `send`, `on`, unsubscribe; 5 tests | ✅ |
| Contract 2: **Presence** — typed per-client state; `track`, `untrack`, `onJoin`, `onLeave`, snapshot; 5 tests | ✅ |
| Contract 3: **Postgres Changes** — `INSERT`, `UPDATE`, `DELETE`; `on`, `onInsert`, `onUpdate`, `onDelete`; 6 tests | ✅ |
| 16 realtime tests total — all three contracts, no live Supabase connection | ✅ |
| `fid dash --portfolio` — reads `fiducial.portfolio` manifest, aggregates `fid dash --json` from each repo | ✅ |
| Portfolio is a view: it writes nothing (verified by test) | ✅ |
| Portfolio reports errors per-product without failing the command — same posture as single-product dash | ✅ |
| `fiducial.portfolio` — `[[products]]` TOML format; name + path per entry | ✅ |
| 5 portfolio end-to-end tests in `tests/portfolio.rs` | ✅ |
| CI: `sim`, `realtime`, `portfolio` jobs | ✅ |
| ~~`fiducial-sim` in spine matrix~~ — **corrected in Phase 18**: it never was, and cannot be. `fiducial-sim` uses `Vec` and rayon and is not `no_std`. It is checked on host and `wasm32` instead | ⬜ |

**Phase 16c — complete ✅ (2026-09-10)**

> Firmware OTA: `fiducial-ota` — the first sub-protocol built on the waist.
> Done when: transfer resumes after a drop; an unsigned image cannot stage; a failed self-test rolls back.

| Deliverable | Status |
| --- | --- |
| `crates/fiducial-ota/` — `no_std`, no `alloc`; builds on all four targets with and without software crypto | ✅ |
| `ImageManifest` — signed metadata; commits to the image by SHA-256, so 43 bytes authenticate a 400 KB image and a bad offer costs zero flash writes | ✅ |
| `signing_bytes()` — fixed-width big-endian canonical form, deliberately **not** postcard: postcard is a serialization, not a canonicalization, and tying signatures to it would let an encoding change silently invalidate deployed keys | ✅ |
| `OtaMessage` — `Offer` / `Resume` / `Chunk` / `Ack` / `Status`, postcard-serialized, carried in a `fiducial-protocol` frame | ✅ |
| `Receiver<V, SLOT>` — pure state machine. Touches no flash, no radio, no clock; that is what makes every §12.3 failure mode a host test | ✅ |
| **Never brick** — `TrialState`: trial boot, boot budget, automatic rollback when unconfirmed | ✅ |
| **Booting ≠ healthy** — `mark_booted(self_test_passed)`; a booted-but-failing image still rolls back | ✅ |
| **Signed, always** — no code path reaches `Phase::Staged` without a verified signature; asserted, not commented | ✅ |
| Default verifier is `RejectAll` — an unconfigured device installs *nothing* rather than *anything* | ✅ |
| `SignatureVerifier` trait — software ed25519, STM32WLE5 hardware PKA, or a test stub. Three real implementations, not a speculative abstraction | ✅ |
| **Resumable** — `resume_offset()`; a 1 KB transfer dropped at 384 bytes continues from 384 | ✅ |
| **Idempotent** — a duplicate chunk is *acknowledged*, not rejected (§12.4); failing there live-locks a flaky link | ✅ |
| **Power-safe** — `PowerPolicy` is a declared fact; a product declaring 80% gets 80% enforced | ✅ |
| **Staged rollout** — `Cohort` is inside the signed bytes, so a canary image cannot be replayed at the fleet | ✅ |
| No downgrade or replay — offers at or below the running version refused | ✅ |
| 21 transfer tests + 5 real-ed25519 signing tests (genuine key signs; wrong key, flipped bit, and every tampered field all refused) | ✅ |
| CI: `ota` job — transfer, signing, and `no_std` on thumbv6m / thumbv7em / wasm32, each with and without `ed25519` | ✅ |

**Phase 16 — the protocol, documented and pinned ✅ (2026-09-10)**

> Closing out Phase 16 altogether: the wire is specified, and conformance is machine-checked.

| Deliverable | Status |
| --- | --- |
| `docs/protocol/README.md` — canonical wire specification: hourglass model, frame layout, decoder state machine, CRC-32 parameters, version negotiation, OTA sub-protocol, implementation checklist | ✅ |
| `docs/protocol/vectors.json` — **conformance vectors**, generated from Rust, committed, asserted by **both** the Rust and TypeScript suites | ✅ |
| Vector cases chosen for properties, not coverage: empty payload, `"ab"` vs `"ba"` (reordering — indistinguishable under the v1 XOR fold), payload containing `MAGIC`, all-zeros/all-ones, lengths straddling the `LEN` byte boundary | ✅ |
| Round-trip closed through the **decoder**, not the encoder — asserting the encoder against itself would be circular | ✅ |
| Freshness gate: CI runs the generator without `FIDUCIAL_WRITE_VECTORS`, so a wire change that skipped regeneration fails the build | ✅ |
| **Anchor verified adversarially** — flipping one bit of the TypeScript CRC polynomial fails 31 tests. The vectors catch drift; they do not merely pass | ✅ |
| Section 8 records what is deliberately *excluded* (encryption, COSE, CBOR, per-frame version byte, fragmentation, retransmission) with the reason, so each stops being re-proposed | ✅ |
| TypeScript suite: 24 → **61** tests | ✅ |

**Phase 16b — complete ✅ (2026-09-10)**

> `fid release` + version-skew assertions.
> Done when: A protocol bump fails any artifact still on the old version; compatibility matrix committed.

| Deliverable | Status |
| --- | --- |
| `WIRE_VERSION: u8 = 1` in `fiducial-protocol` — single declaration of the wire protocol version, embedded in every compiled artifact | ✅ |
| `assert_compatible(local, remote, min_compatible)` — rejects a remote below the declared minimum; returns `VersionSkewError` with all three fields | ✅ |
| `is_current_compatible(remote)` — convenience form using `WIRE_VERSION` as both local and min | ✅ |
| `core::error::Error` impl for `VersionSkewError` — usable in `?` chains on any Rust target (MSRV 1.82) | ✅ |
| `docs/compat/matrix.toml` — committed compatibility policy: `wire.current`, `wire.min_compatible`, dated history | ✅ |
| `fid release status` — print platform CLI version, WIRE_VERSION, and matrix | ✅ |
| `fid release check` — fail when `matrix.wire.current ≠ WIRE_VERSION`; the CI enforcement point | ✅ |
| `fid release protocol --bump breaking` — increment wire version, raise min_compatible, print ACTION REQUIRED reminder | ✅ |
| `fid release protocol --bump compatible` — increment wire version, keep min_compatible, print reminder | ✅ |
| 5 unit tests in `commands::release` + 14 version-skew tests in `fiducial-protocol` | ✅ |
| 6 end-to-end tests in `tests/release.rs` — covers happy-path check, stale-matrix rejection, breaking bump, compatible bump, old-artifact rejection | ✅ |
| `release` CI job — protocol tests + integration tests + `fid release check` on the committed matrix | ✅ |
| `docs/specs/2026-09-10-release-and-version-skew.md` — decision record | ✅ |
| "Done when" criterion: `assert_compatible(2, 1, 2).is_err()` proves an artifact on v1 is rejected after a breaking bump to v2 | ✅ |

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
├── SHIPPED.md           ← this file
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
| **16** | Workbench v0: `fid dash` read-only view | Roadmap, status, decisions, CI, graph, freshness in one place | ✅ |
| **16b** | `fid release` + version-skew assertions | A protocol bump fails any artifact still on the old version; compatibility matrix committed | ✅ |
| **16c** | Firmware OTA: `fiducial-ota` — signed manifests, resumable transfer, trial boot, staged rollout | Transfer resumes after a drop; unsigned image cannot stage; failed self-test rolls back ([rescoped from BLE](docs/specs/2026-09-10-phase-16c-ota-transport-rescope.md)) | ✅ |
| **17** | `fiducial-sim`, `realtime`, workbench v1 | Simulation runs native and in WASM; realtime's three contracts covered by tests | ✅ |
| **18** | Platform audit + cleanup | Every drift found is encoded as an invariant that fails the build | ✅ |
| **19** | The harvest feature — `fid harvest` + skill + docs | A repo or folder yields reusable assets a new product can adopt without being overridden | ✅ |
| **20** | Catalogue ROP's reusable assets | Every reusable asset inventoried with an extraction recipe | ✅ |
| **21** | The documentation layer | Step-by-step guides for humans and agents; terminal captures checked by CI | ✅ |
| **21b** | Namespaced agents — `fiducial-design` / `-review` / `-implement` | A subagent's filename is its identity; a colliding name is silently unavailable | ✅ |
| **22** | i18n — localized by construction | A missing translation fails `fid derive --check` | ✅ |

**What comes next is not recorded here.** This file is the record of what was
*built*; [`ROADMAP.md`](ROADMAP.md) holds what is *intended* and in what order.
Naming the next items in both places was one fact declared twice — briefly gated
by a test, now simply removed, because a gated duplicate is still a duplicate.

*(ROP migration wave 2 — eligible rules to L0 Rust — remains optional and
unscheduled, as wave 1 does.)*

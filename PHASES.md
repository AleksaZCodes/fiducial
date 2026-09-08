# Fiducial — Build Order & Current State

_Read this at the start of every session. Updated manually as phases complete._

---

## Current phase: Phase 12

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
│   ├── fiducial-wasm/       wasm-bindgen bindings → fiducial-core
│   └── fiducial-cli/        `fid` binary — new, add, doctor, guard-check
├── firmware/                SEPARATE Cargo workspace (excluded from root)
│   ├── Cargo.toml           workspace root
│   ├── rust-toolchain.toml  stable + embedded targets
│   ├── shared/              target-agnostic helpers (blink constants, re-exports)
│   └── rp2040/              Embassy blink demo — thumbv6m-none-eabi
├── packages/
│   ├── fiducial/            @fiducial/fiducial npm placeholder
│   ├── cli/                 @fiducial/cli npm shim for `fid`
│   ├── tokens/              @fiducial/tokens — L1 design tokens + Tailwind preset
│   └── headless/            @fiducial/headless — L2 Result<T,E> + OfflineQueue<T>
└── docs/
    └── specs/
        └── 2026-09-06-fiducial-design.md
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
| **12** | `firmware/rp2040` + `stm32`, probe-rs + defmt, LoRa via `lora-rs` | `fid add firmware` yields a flashable project sharing L0 with the desktop app | ⬜ |
| **13** | Web Serial/WebUSB + BLE transports | Same device reachable from browser and phone with the same codec | ⬜ |
| **14** | EDA pipeline: atopile → KiCad → `board.interface.json` + fab outputs | A board change regenerates every output; `--check` catches staleness | ⬜ |
| **15** | `fiducial-geometry` + `fiducial-mesh` + `viewer3d-*` + tolerance profiles | Board outline → generated enclosure → printable STL and a GLB on a marketing page | ⬜ |
| **16** | Workbench v0: `fid dash` read-only view | Roadmap, status, decisions, CI, graph, freshness in one place | ⬜ |
| **16b** | `fid release` + version-skew assertions | A protocol bump fails any artifact still on the old version; compatibility matrix committed | ⬜ |
| **16c** | Firmware OTA: `embassy-boot` A/B, signing, resumable transfer, staged rollout | Device updates over BLE, self-tests, marks booted — broken image rolls back automatically | ⬜ |
| **17** | `fiducial-sim`, `realtime`, workbench v1 | Simulation runs native and in WASM; realtime's three contracts covered by tests | ⬜ |
| **18** | ROP migration wave 2 (optional) — eligible rules to L0 Rust | Each differential-tested before the TypeScript is deleted | ⬜ |

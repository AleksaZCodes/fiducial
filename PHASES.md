# Fiducial — Build Order & Current State

_Read this at the start of every session. Updated manually as phases complete._

---

## Current phase: Phase 3

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

**Phase 3 — not started**

> `fid` CLI: `new`, `add`, `doctor`; `fiducial.lock`; guard configurable and shell-aware.
> Done when `fid new` yields a building repo, `fid doctor` is clean, and the §11 false-positive cannot recur.

Spec: `docs/specs/2026-09-06-fiducial-design.md` §16, Phase 3 row.

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
│   └── fiducial-wasm/       wasm-bindgen bindings → fiducial-core
├── firmware/                SEPARATE Cargo workspace (excluded from root)
│   ├── Cargo.toml           workspace root
│   ├── rust-toolchain.toml  stable + embedded targets
│   ├── shared/              target-agnostic helpers (blink constants, re-exports)
│   └── rp2040/              Embassy blink demo — thumbv6m-none-eabi
├── packages/
│   └── fiducial/            @fiducial/fiducial npm placeholder
└── docs/
    └── specs/
        └── 2026-09-06-fiducial-design.md
```

---

## Source material (local, not in this repo)

- `/home/aleksa/dev/rop-reference/` — ROP snapshot at b16b226; characterization baseline for Phase 6. Never modify, never push (contains real secrets).
- `/home/aleksa/dev/platform/` — retired planning folder. Superseded by this repo.

---

## Phase build order (spec §16)

| Phase | Deliverable | Done when | Status |
| --- | --- | --- | --- |
| **0** | Claim names; repo; MISSION, LICENSE, IP-POLICY | Both publish --dry-run pass | ✅ |
| **1** | Monorepo skeleton: Turborepo, Changesets, CI | Trivial package publishes and installs in scratch project | ✅ |
| **2** | Vertical slice: `fiducial-core` no_std + WASM + 4-target CI | One fn runs in browser, Tauri, and blinks LED on RP2040 | ✅ |
| **3** | `fid` CLI: `new`, `add`, `doctor`; shell-aware guard | `fid new` builds; `fid doctor` clean; false-positive cannot recur | ⬜ next |
| **3b** | Capability mechanism | A capability installs, activates guard + skill | ⬜ |
| **4** | Propagation: codemods + 3-way template merge | Upstream change lands in product, conflict surfaced correctly | ⬜ |
| **5** | Claude Code plugin | Six-line block → guard active in scratch repo | ⬜ |
| **6** | ROP characterization harness | Green on unmodified ROP; fails on injected change | ⬜ |
| **7** | L1 tokens + L2 packages | Each publishes and typechecks in isolation | ⬜ |
| **8** | ROP migration wave 1 (optional, on your schedule) | Every PR green on all tiers + characterization | ⬜ |

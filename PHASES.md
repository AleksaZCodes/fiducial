# Fiducial — Build Order & Current State

_Read this at the start of every session. Updated manually as phases complete._

---

## Current phase: Phase 7

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
│   └── cli/                 @fiducial/cli npm shim for `fid`
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
| **7** | L1 tokens + L2 packages | Each publishes and typechecks in isolation | ⬜ |
| **8** | ROP migration wave 1 (optional, on your schedule) | Every PR green on all tiers + characterization | ⬜ |

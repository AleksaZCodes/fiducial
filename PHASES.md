# Fiducial — Build Order & Current State

_Read this at the start of every session. Updated manually as phases complete._

---

## Current phase: Phase 1

**Phase 0 — complete ✅ (2026-09-06)**

| Deliverable | Status |
| --- | --- |
| Repo `AleksaZCodes/fiducial` (public, MIT) | ✅ https://github.com/AleksaZCodes/fiducial |
| `MISSION.md`, `STACK.md`, `LICENSE`, `IP-POLICY.md` | ✅ |
| `crates/fiducial` v0.1.0 → crates.io | ✅ published |
| `@fiducial/fiducial` v0.1.0 → js registry (`@fiducial` org created) | ✅ published |
| Design spec → `docs/specs/2026-09-06-fiducial-design.md` | ✅ |

**Phase 1 — not started**

> Monorepo skeleton: Turborepo, Changesets, host Cargo workspace, release CI.
> Done when a trivial package publishes and installs in a scratch project.

Spec: `docs/specs/2026-09-06-fiducial-design.md` §16, Phase 1 row.

---

## Repo layout (current)

```
fiducial/
├── MISSION.md
├── STACK.md
├── LICENSE             MIT
├── IP-POLICY.md
├── README.md
├── PHASES.md           ← this file
├── Cargo.toml          host workspace
├── package.json        pnpm workspace root (private)
├── pnpm-workspace.yaml
├── .gitignore
├── crates/
│   └── fiducial/       crates.io placeholder
├── packages/
│   └── fiducial/       @fiducial/fiducial js registry placeholder
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
| **1** | Monorepo skeleton: Turborepo, Changesets, CI | Trivial package publishes and installs in scratch project | ⬜ next |
| **2** | Vertical slice: `fiducial-core` no_std + WASM + 4-target CI | One fn runs in browser, Tauri, and blinks LED on RP2040 | ⬜ |
| **3** | `fid` CLI: `new`, `add`, `doctor`; shell-aware guard | `fid new` builds; `fid doctor` clean; false-positive cannot recur | ⬜ |
| **3b** | Capability mechanism | A capability installs, activates guard + skill | ⬜ |
| **4** | Propagation: codemods + 3-way template merge | Upstream change lands in product, conflict surfaced correctly | ⬜ |
| **5** | Claude Code plugin | Six-line block → guard active in scratch repo | ⬜ |
| **6** | ROP characterization harness | Green on unmodified ROP; fails on injected change | ⬜ |
| **7** | L1 tokens + L2 packages | Each publishes and typechecks in isolation | ⬜ |
| **8** | ROP migration wave 1 (optional, on your schedule) | Every PR green on all tiers + characterization | ⬜ |

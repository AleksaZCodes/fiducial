# Fiducial — Agent context

> One declaration, many derivations. See `MISSION.md` for the why.

This file covers the same ground as `CLAUDE.md` for agent runtimes that read
`AGENTS.md` (Codex, Copilot Workspace, etc.). The authoritative design is in
`docs/specs/2026-09-06-fiducial-design.md`.

## Current phase

Read `SHIPPED.md` before every session. It states which phase is active and what
"done" means for it. Do not begin the next phase until the current one is
complete.

## Principles

**Read [`MISSION.md`](MISSION.md).** It is in this repository, it is where the
principles are authored, and there is no summary of it here on purpose — a
summary is a second declaration that drifts from the thing it summarises.

(A scaffolded *product* does get the text, because its own `MISSION.md` states
what the product is for rather than the platform's rules. It is generated into
the product's `AGENTS.md` from this file at scaffold time — see
`crates/fiducial-cli/build.rs`.)

## Repo layout## Repo layout

```
fiducial/
├── MISSION.md, STACK.md, SHIPPED.md
├── crates/               13 Rust members
│   ├── fiducial-core         no_std spine — DeviceId, version
│   ├── fiducial-protocol     no_std frame codec — the waist
│   ├── fiducial-quantity     no_std units + tolerance algebra
│   ├── fiducial-model        Fact, Decision, PipelineMeta
│   ├── fiducial-geometry     no_std primitives, tolerance profiles
│   ├── fiducial-mesh         no_std case generation, STL + GLB
│   ├── fiducial-eda          board.interface.json schema + validation
│   ├── fiducial-ota          no_std signed, resumable firmware update
│   ├── fiducial-sim          ODE simulation — native (rayon) + WASM
│   ├── fiducial-cli          the `fid` binary + capability templates
│   ├── fiducial-wasm         wasm-bindgen wrapper
│   ├── fiducial-tauri        serial transport for the desktop host
│   └── fiducial              crates.io name claim + signpost
├── firmware/             Separate workspace (Embassy; rp2040 + stm32)
├── packages/             12 JS/TS packages (pnpm + Turborepo)
│   ├── tokens, headless      design tokens, Result<T,E>, OfflineQueue
│   ├── ui-react, ui-svelte   component registry sources (copy-in)
│   ├── board-schema          TS mirror of fiducial-eda
│   ├── transport-web         Web Serial / WebUSB / BLE + codec
│   ├── viewer3d-react        GLB viewer (Three.js)
│   ├── realtime              broadcast, presence, postgres-changes
│   ├── i18n                  locales, messages, money, dates, plurals
│   ├── wasm-bridge           GENERATED TS types — never hand-edit
│   ├── cli                   @fiducial/cli npm shim
│   └── fiducial              @fiducial/fiducial npm name claim
├── docs/
│   ├── specs/            Design decisions (append-only, date-prefixed)
│   ├── protocol/         Wire spec + conformance vectors
│   └── compat/           Wire version policy (matrix.toml)
└── .github/workflows/    CI (ci.yml) + release (release.yml)
```

This tree is checked by `crates/fiducial-cli/tests/workspace_hygiene.rs`, which
fails the build when a crate or package is missing from it. It used to carry a
disclaimer telling you to run `ls` instead; checking is cheaper than
disclaiming.

## Tools — use these before writing library code

### context7 (live documentation)

Before writing code against any named library — Embassy, wasm-bindgen, wasm-pack,
Turborepo, Changesets, Tauri, probe-rs, Next.js, SvelteKit, and anything else with a
versioned API — fetch current docs via context7. Training data goes stale; library
feature names change. The Phase 2 Embassy renames (`arch-cortex-m` → `platform-cortex-m`,
`integrated-timers` removed, `embassy-rp` jumping from 0.3 to 0.10) cost a full
trial-and-error loop because context7 was not used. That is avoidable.

Step 1: `resolve-library-id` with the library name and a topic query.
Step 2: `query-docs` with the returned ID and a specific question.

### Claude Code skills

`/code-review`, `/commit`, `/commit-push-pr`, `/run`, `/security-review`,
`/update-config`, `/claude-api`. Invoke by name or via the Skill tool.

### GitHub MCP plugin

`mcp__plugin_github_github__*` — search code, read files, create PRs, manage issues.
Use `get_me` first to confirm current user context.

## Documentation

| Read | For |
|---|---|
| `docs/guides/for-agents.md` | **Working here as an agent — start with this** |
| `docs/guides/start-here.md` | The paradigm, and what it changes about how you work |
| `docs/guides/first-product.md` | Nothing → board → generated enclosure → CI gate |
| `docs/guides/harvesting.md` | Getting the good parts out of a codebase already built |

Terminal output in those guides is **generated from the real binary** and gated
in CI. Never hand-edit a block showing `fid` output — regenerate it with
`FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures`.

## Reusing an existing codebase

```sh
fid harvest <path> --name <slug>   # survey + stage
```

`harvest/` is a **staging area and never the product**. Nothing is wired in and
nothing is overwritten. Do not paste donor files into the source tree —
generalize them deliberately, or you have imported somebody else's assumptions
along with their work. See `docs/guides/harvesting.md`.

## Build

```sh
pnpm build       # JS workspace (turbo)
cargo build      # Rust workspace
cargo test && pnpm test
```

## The `fid` commands

Listed here rather than only in a Claude Code skill, because this file is the
context **every** agent reads — Codex, Copilot Workspace and Cursor included.
An agent that does not know `fid derive --check` exists cannot honour the one
rule that matters most.

| Command | Does |
|---|---|
| `fid dash [--json]` | The whole product in one view. `--json` is for you. Start here |
| `fid doctor` | Config, lock, template integrity, pending migrations |
| `fid derive` | Run the pipelines; record every artifact hash |
| `fid derive --check` | **Fail when an artifact drifted from its declaration.** The CI gate |
| `fid graph` | Every pipeline: inputs → executor → outputs. Use it to find which declaration produced a file |
| `fid new <name>` | Scaffold a product |
| `fid add <capability>` | Install a capability |
| `fid upgrade [--dry-run]` | 3-way merge upstream template changes; apply codemods |
| `fid harvest <path>` | Survey another codebase for reusable work |
| `fid release check` | Fail when the compatibility matrix and `WIRE_VERSION` disagree |
| `fid capability list` | What is installed and what is available |

Every command takes `--help`, and the help names no phase numbers on purpose —
a schedule is a fact `SHIPPED.md` owns, and a second copy of it drifts.

## Skills this repository authors

| File | Invoked as (Claude Code) | Does |
|---|---|---|
| `commands/platform.md` | `/fiducial:platform` | Load platform context at session start |
| `commands/harvest.md` | `/fiducial:harvest` | Extract reusable work from another codebase |
| `crates/fiducial-cli/capabilities/*/SKILL.md` | installed per capability | How to use that capability |

**The content is portable; only discovery is not.** These are plain Markdown
instructions — an agent without Claude Code's slash commands can read the file
directly and follow it. Capability instructions install to
`.fiducial/skills/<id>.md` in a product for exactly that reason, with a pointer
at `.claude/skills/<id>.md` for Claude's auto-discovery.

Generating the per-vendor wrappers from one authored source is roadmap item
**agent portability**; today the wrapper for Claude is written by hand and there
is none for anyone else.

## What not to do

- Do not modify `/home/aleksa/dev/rop-reference/` (real secrets).
- Do not touch ROP production code before Phase 6 is green.
- Do not publish or push the rop-reference directory.

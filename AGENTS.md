# Fiducial — Agent context

> One declaration, many derivations. See `MISSION.md` for the why.

This file covers the same ground as `CLAUDE.md` for agent runtimes that read
`AGENTS.md` (Codex, Copilot Workspace, etc.). The authoritative design is in
`docs/specs/2026-09-06-fiducial-design.md`.

## Where the work is recorded

Two files, and the names say which is which:

| File | Holds | Read it for |
|---|---|---|
| [`ROADMAP.md`](ROADMAP.md) | what is **intended**, ordered, with ⬜ 🟡 ✅ markers | what to do next |
| [`SHIPPED.md`](SHIPPED.md) | what was **built**, phase by phase | what already exists, and what "done" meant for it |

Read both before starting. **Neither file states "what is next" in prose** — the
roadmap's order plus its markers already do, and `fid dash` derives it:

```sh
fid dash --section roadmap    # progress, anything in flight, and the next item
```

A sentence naming the next item is a second declaration of those markers, and it
is the copy that goes stale. One lived in `SHIPPED.md` until 2026-09-15 and had
to be hand-edited on every merge.

Finish the item you are on before starting the next.

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

A fresh clone builds with **no setup step**. Verified by cloning and running
cold: 375 Rust tests, 29 JS tasks, both freshness gates.

```sh
pnpm install --frozen-lockfile
pnpm build && pnpm typecheck && pnpm test     # JS workspace (turbo)
cargo build --workspace
cargo test --workspace --all-features         # includes the freshness gates
```

Everything that decides *how* it builds is committed, so a cloud checkout — Claude
Code on the web, a Codespace, a new contributor — gets the same answers as a
laptop:

| Pinned by | What |
|---|---|
| `rust-toolchain.toml` | channel, `rustfmt`/`clippy`, **and the three cross-compilation targets** the spine check needs |
| `packageManager` in `package.json` | the exact pnpm version |
| `.nvmrc` + `engines` | Node |
| `pnpm-lock.yaml` + `Cargo.lock` | every dependency |
| `.claude/settings.json` | the plugin, so the guard is active |

The targets line matters: without it, `cargo check --target wasm32-unknown-unknown`
fails on a fresh machine and looks like a code problem rather than a missing
`rustup target add`.

**What a cloud session does not get, by design:** `.claude/settings.local.json`
is gitignored because it holds personal tool permissions. Its absence means more
permission prompts, not a broken environment.

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
| `fid capability list [--all]` | What each capability declares, derives and requires; `--all` also lists the adapter contracts |
| `fid add capability <id> [--from <source>]` | Install a capability, built-in or resolved from a directory or git repository |

Every command takes `--help`, and the help names no phase numbers on purpose —
a schedule is a fact `SHIPPED.md` owns, and a second copy of it drifts.

## What a capability is made of

Four kinds, and the difference is not cosmetic — see
`docs/specs/2026-09-14-capability-taxonomy.md`.

| Kind | Is | Example |
|---|---|---|
| **Declaration** | a typed fact, written once, inert | `board/board.interface.json`, the `[i18n]` block |
| **Pipeline** | reads declarations, produces artifacts, **gated by `fid derive --check`** | `pipelines/eda.toml` |
| **Adapter** | a swappable vendor behind a fixed contract, selected in `[adapters]` | `storage = "none"` |
| **Template** | a plain file copied in, belonging to no pipeline | `apps/worker/wrangler.toml` |

The test for a declaration: *could two different pipelines read this and both be
correct?* If yes it is a declaration; if it is one tool's config file it is a
template.

Filing one as another is not a style mistake. A pipeline outside `pipelines/` is
installed and never runs; a `pipelines/` file listed as a template is installed
and never gated. `fid capability check` rejects both.

**A capability is a directory, and its manifest is derived from it.** `SKILL.md`
is the only required file; `declarations/` and `pipelines/` say what each file
is; `capability.toml` is optional and carries only what a layout cannot. A
capability need not be compiled into `fid` — `--from <path>` or
`--from git:<url>#<rev>` installs one through the same derivation, the same
conformance checks and the same lock entry as a built-in.

**Adapters name contracts, not vendors.** Every contract currently implements
only `none` — a real, working no-op. The vendors each is intended to carry are
listed as *planned* and cannot be selected, because a selectable name with
nothing behind it is a promise the platform does not keep.

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

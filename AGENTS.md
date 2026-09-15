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

## Repo layout

<!-- fid:begin layout -->
```
fiducial/
├── crates/               15 members
│   ├── fiducial                 Declare each fact once. Derive every artifact from it.
│   ├── fiducial-adapters        Cross-platform adapter contracts for Fiducial — database, s…
│   ├── fiducial-cli             fid — the Fiducial platform CLI
│   ├── fiducial-core            no_std spine: IDs, time primitives, validation, state machi…
│   ├── fiducial-eda             no_std EDA pipeline types — the BoardInterface schema for b…
│   ├── fiducial-geometry        no_std geometry for Fiducial: points, tolerance profiles, c…
│   ├── fiducial-identity        no_std identity spine: one Principal across users, devices…
│   ├── fiducial-mesh            no_std case generation from a board outline: gasket-sealed…
│   ├── fiducial-model           no_std Fact, Decision and Pipeline contract types.
│   ├── fiducial-ota             no_std over-the-air firmware update protocol: signed manife…
│   ├── fiducial-protocol        no_std transport-agnostic protocol: framing, checksums, seq…
│   ├── fiducial-quantity        no_std typed quantities with tolerance algebra and assertions.
│   ├── fiducial-sim             Numerical simulation — ODE integration that runs native (wi…
│   ├── fiducial-tauri           Serial transport and device discovery for Fiducial Tauri apps.
│   └── fiducial-wasm            WASM bindings for fiducial-core — browser, edge, and Cloudf…
└── packages/             14 members
    ├── adapters                 Cross-platform adapter contracts for Fiducial — database, s…
    ├── board-schema             TypeScript types for board.interface.json — mirrors the Rus…
    ├── cli                      fid — the Fiducial platform CLI
    ├── fiducial                 Declare each fact once. Derive every artifact from it.
    ├── headless                 Framework-agnostic headless utilities for Fiducial — Result…
    ├── i18n                     Localized by construction — typed message keys, locale nego…
    ├── identity                 One identity model across users, devices and services — mir…
    ├── realtime                 Supabase Realtime typed wrappers — Broadcast, Presence, and…
    ├── tokens                   Design tokens for Fiducial — OKLCH theme vars, Tailwind v4…
    ├── transport-web            Web Serial, WebUSB, and BLE transports for Fiducial — same…
    ├── ui-react                 Fiducial component registry source — React. Use `fid add co…
    ├── ui-svelte                Fiducial component registry source — Svelte. Use `fid add c…
    ├── viewer3d-react           React component for rendering Fiducial board GLB files in a…
    └── wasm-bridge              Generated TypeScript types for the fiducial WASM boundary.
```
<!-- fid:end layout -->

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

## Documentation that cannot go stale quietly

Three mechanisms, and between them they cover the whole surface:

| What | Gate | Fix when it fails |
|---|---|---|
| Derived blocks (`<!-- fid:begin … -->`) | `fid context --check` | `fid context` |
| Terminal output in the guides | `cargo test -p fiducial-cli --test captures` | `FIDUCIAL_WRITE_CAPTURES=1 cargo test …` |
| **Hand-written prose** | `fid docs --check` | read it, fix it, `fid docs --accept` |

The third is the one that is not automatic, because it cannot be. A paragraph
explaining *why* something is shaped a certain way cannot be generated from the
thing it explains — if it could, it would carry nothing the code does not.

So what is derived is **the obligation to revisit it.** A prose block names the
source it describes:

```markdown
<!-- fid:describes crates/fiducial-cli/src/adapter.rs#pub static CONTRACTS -->
Every contract ships with `none` — a real, working no-op…
<!-- fid:end-describes -->
```

`docs/prose.lock` records that source's hash as it stood when someone last read
the paragraph against it. When it moves and the paragraph does not, the gate
fails and names both.

**`fid docs --accept` means "I have read this against its source."** Running it
to make a red build green, without reading, is the one thing that makes the
mechanism worthless — it is a separate command from `--check` for that reason.
Narrow a block with `#Symbol`: a whole-file watch fires on every unrelated edit,
and a gate that cries wolf trains you to accept without reading.

Spec: `docs/specs/2026-09-15-prose-is-gated-not-generated.md`.

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

Three suites run **real generated output**, so they need something built first
and are not part of the default run:

```sh
cargo build -p fiducial-cli --bin fid
pnpm --filter @fiducial/identity build            # the tests import ../dist
pnpm --filter @fiducial/identity test:schema      # the schema on real SQLite

PGHOST=localhost PGUSER=postgres PGPASSWORD=postgres scripts/verify-postgres.sh

pnpm --filter @fiducial/adapters build
pnpm --filter @fiducial/adapters test:generated   # the factory through real tsc
```

`test:schema` generates the grants table with the real `fid` binary and runs
it on `node:sqlite` — D1 *is* SQLite. `verify-postgres.sh` applies the same
derivation, RLS policies included, to a real PostgreSQL server.
`test:generated` derives `src/adapters.generated.ts` for every vendor selection
and compiles it with `tsc`. The `identity` and `adapters` CI jobs run them; the
generic JS/TS job builds no Rust and cannot.

All three exist for one reason: **generated code asserted as text is not
tested.** A `contains()` check passes on a migration no server will run and on
a factory no compiler will accept, and both of those shipped here before these
suites existed.

They are separate scripts rather than part of `pnpm test` because a suite that
cannot run is worse than one that is named: the first CI run of these failed
on a missing binary and a missing `dist/`, saying nothing about either.

Everything that decides *how* it builds is committed, so a cloud checkout — Claude
Code on the web, a Codespace, a new contributor — gets the same answers as a
laptop:

<!-- fid:describes rust-toolchain.toml -->

| Pinned by | What |
|---|---|
| `rust-toolchain.toml` | channel, `rustfmt`/`clippy`, **and the three cross-compilation targets** the spine check needs |
| `packageManager` in `package.json` | the exact pnpm version |
| `.nvmrc` + `engines` | Node |
| `pnpm-lock.yaml` + `Cargo.lock` | every dependency |
| `.claude/settings.json` | the plugin, so the guard is active |

<!-- fid:end-describes -->

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

<!-- fid:begin commands -->
| Command | Does |
|---|---|
| `fid new` | Scaffold a new product repository |
| `fid add` | Add an app, module, or firmware target to this product |
| `fid capability` | Manage capabilities installed in this product |
| `fid derive` | Run declared pipelines (`--check` fails CI on stale artifacts) |
| `fid upgrade` | Pull upstream template and package updates into this product |
| `fid graph` | Emit the facts → pipelines → artifacts dependency graph |
| `fid release` | Platform version management and version-skew enforcement |
| `fid dash` | The workbench — one read-only view of roadmap, decisions, CI, graph, freshness |
| `fid harvest` | Survey an existing codebase for reusable logic, art, UI and principles |
| `fid context` | Regenerate the derivable parts of AGENTS.md / CLAUDE.md |
| `fid docs` | Documentation freshness, including prose nothing can generate |
| `fid doctor` | Check for drift: outdated deps, stale templates, un-applied migrations |
<!-- fid:end commands -->

Every command takes `--help`, and the help names no phase numbers on purpose —
a schedule is a fact `SHIPPED.md` owns, and a second copy of it drifts.

## What a capability is made of

Four kinds, and the difference is not cosmetic — see
`docs/specs/2026-09-14-capability-taxonomy.md`.

<!-- fid:describes crates/fiducial-cli/src/capability/manifest.rs#pub fn derive -->

| Kind | Is | Example |
|---|---|---|
| **Declaration** | a typed fact, written once, inert — a file, a `fiducial.toml` block, or a directory the product fills | `board/board.interface.json`, the `[i18n]` block, `migrations/` |
| **Pipeline** | reads declarations, produces artifacts, **gated by `fid derive --check`** | `pipelines/eda.toml` |
| **Adapter** | a swappable vendor behind a fixed contract, selected in `[adapters]` | `storage = "none"` |
| **Template** | a plain file copied in, belonging to no pipeline | `apps/worker/wrangler.toml` |

<!-- fid:end-describes -->

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

The capabilities this platform ships:

<!-- fid:begin capabilities -->
| Capability | Contributes | Install |
|---|---|---|
| `adapters` | declares `adapters`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability adapters` |
| `brand` | declares `brand`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability brand` |
| `deploy` | declares `deploy`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability deploy` |
| `eda` | declares `board/board.interface.json`; 2 pipeline(s); 1 template file(s) | `fid add capability eda` |
| `firmware-rp2040` | 10 template file(s) | `fid add capability firmware-rp2040` |
| `firmware-stm32` | 9 template file(s) | `fid add capability firmware-stm32` |
| `i18n` | declares `i18n`, `messages/en.json`, `messages/sr.json`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability i18n` |
| `identity` | declares `identity`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability identity` |
| `migrations` | declares `migrations`; 1 pipeline(s) | `fid add capability migrations` |
| `tauri` | 5 template file(s) | `fid add capability tauri` |
| `web-next` | 8 template file(s) | `fid add capability web-next` |
| `web-svelte` | 8 template file(s) | `fid add capability web-svelte` |
| `worker-cloudflare` | 1 template file(s) | `fid add capability worker-cloudflare` |
<!-- fid:end capabilities -->

**Adapters name contracts, not vendors.** A product picks a vendor per contract
in `[adapters]`, and every contract ships with `none` — a real, working no-op,
not a placeholder, which is what makes it cost nothing to wire in on day one.

A name under **Planned** cannot be selected and fails with a message saying so,
because a selectable name with nothing behind it is a promise the platform does
not keep. This table is generated from the registry that enforces that rule:

<!-- fid:begin adapters -->
| Contract | For | Selectable today | Planned |
|---|---|---|---|
| `database` | Relational storage: queries, migrations, transactions | `none`, `d1` | `supabase`, `neon`, `postgres` |
| `storage` | Object storage: put, get, signed URLs | `none`, `r2` | `s3`, `supabase-storage` |
| `deploy` | Where the product ships and how a release is promoted | `none`, `cloudflare` | `vercel`, `fly` |
| `email` | Transactional email: send, template, verify a domain | `none` | `resend`, `ses`, `cloudflare-email` |
| `errors` | Error tracking and diagnostics | `none` | `sentry`, `workers-analytics` |
| `botProtection` | Bot / abuse challenge verification | `none`, `turnstile` | `recaptcha`, `hcaptcha` |
| `queue` | Asynchronous job/message queue (producer side) | `none`, `cloudflare-queues` | `sqs` |
| `auth` | Users and authentication: sign-up, sign-in, sessions | `none`, `supabase` | `clerk`, `auth.js` |
<!-- fid:end adapters -->

## Skills this repository authors

<!-- fid:begin skills -->
| File | Invoked as (Claude Code) | Does |
|---|---|---|
| `commands/harvest.md` | `/fiducial:harvest` | Translate an existing codebase into reusable assets — extra… |
| `commands/platform.md` | `/fiducial:platform` | Load Fiducial platform context — run at session start in an… |
| `crates/fiducial-cli/capabilities/*/SKILL.md` | installed per capability | How to use that capability |
<!-- fid:end skills -->

**The content is portable; only discovery is not.** These are plain Markdown
instructions — an agent without Claude Code's slash commands can read the file
directly and follow it. Capability instructions install to
`.fiducial/skills/<id>.md` in a product for exactly that reason, with a pointer
at `.claude/skills/<id>.md` for Claude's auto-discovery.

Generating the per-vendor wrappers from one authored source is roadmap item
**agent portability**; today the wrapper for Claude is written by hand and there
is none for anyone else.

## Contributions from outside

<!-- fid:describes .github/cla-exempt.txt -->

`CLA.md` binds outside contributors so the project keeps the option to
relicense; `IP-POLICY.md` rule 3 is where that requirement is authored. Every
commit from a non-maintainer needs `Signed-off-by:` matching its author, gated
by `outside_contributions_carry_a_cla_sign_off` in
`crates/fiducial-cli/tests/commit_hygiene.rs`. Maintainers and automation are
listed in `.github/cla-exempt.txt`, which is the only place that decides who is
exempt.

<!-- fid:end-describes -->

## What not to do

- Do not modify `/home/aleksa/dev/rop-reference/` (real secrets).
- Do not touch ROP production code before Phase 6 is green.
- Do not publish or push the rop-reference directory.

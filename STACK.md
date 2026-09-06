# STACK.md — the decided stack

**Canonical, per domain. Copied into every product repo; a product may extend it but not
silently contradict it.** Companion to `MISSION.md` (why) and the design spec (how).

Every row carries a status, and the statuses are honest:

- **Decided** — committed. Changing it is an ADR, not a preference.
- **Candidate** — leaning, with the phase where it gets settled named.
- **Gap** — no answer yet, deliberately. Not an oversight.

> **Rule inherited from Ring of Pursuit:** before writing code against any library, framework or
> service, check this table. No row → look for an MCP server or agent skill, add the row, *then*
> start.

---

## 0 · Foundation

| Concern | Decision | Status |
| --- | --- | --- |
| JS package manager | **pnpm**, pinned via Corepack. Never npm/yarn | Decided |
| JS monorepo | **Turborepo** | Decided |
| Rust workspaces | **Cargo** — one host workspace; **separate workspaces per firmware target** | Decided |
| Python (build-time only) | **uv** | Decided |
| Repo layout | `crates/` · `packages/` · `pipelines/` · `apps/` · `firmware/` · `product/` | Decided |
| Version control | Git, branch + PR always, never direct to default branch | Decided |

## 1 · Languages

| Concern | Decision | Status |
| --- | --- | --- |
| Shared logic, everything cross-runtime | **Rust**, `#![no_std]` + `alloc`/`std` features | Decided |
| UI, routing, server glue | **TypeScript** | Decided |
| Pipeline executors where a mature tool is Python | **Python via `uv`** — build-time only, never in the spine or a runtime path | Decided |
| Ruby / Rails | Designed-for (reachable via L0 + tokens), not built | Deferred |

## 2 · The Rust spine (L0)

| Concern | Decision | Status |
| --- | --- | --- |
| Units + dimensional analysis | **`uom`** (alt: `dimensioned`) | Candidate → Phase 9 |
| Serialization | **`serde` + `postcard`** — compact, `no_std`, zero-alloc capable | Decided |
| Rust → JS boundary | **`wasm-bindgen`** + **`ts-rs`** for generated types. Hand-written cross-boundary types are guard-blocked | Decided |
| Parallelism | **`rayon`** native. WASM needs `wasm-bindgen-rayon` + COOP/COEP; single-threaded by default | Decided |
| Targets compiled every commit | `x86_64` · `wasm32-unknown-unknown` · `thumbv6m-none-eabi` · `thumbv7em-none-eabihf` · `thumbv8m.main-none-eabihf` · `riscv32imac-unknown-none-elf` (+ `xtensa-esp32s3-none-elf` when an Xtensa part is in use) | Decided |

## 3 · Web

| Concern | Decision | Status |
| --- | --- | --- |
| SSR framework | **Next.js** (App Router) | Decided |
| Second framework | **SvelteKit** | Decided |
| Static / PWA | Next.js `output: "export"` on Cloudflare Workers Static Assets | Decided |
| Styling | **Tailwind v4**, CSS-first (`@theme`, no config file) | Decided |
| Design tokens | `@fiducial/tokens` — OKLCH palette as CSS custom properties | Decided |
| React primitives | **Base UI** via shadcn conventions | Decided |
| Svelte primitives | **shadcn-svelte / Melt** | Candidate → Phase 10 |
| Component distribution | Package vs shadcn-style copy-in registry | Candidate → Phase 10 |
| Forms | react-hook-form + Zod (React) | Decided |
| i18n | `@fiducial/i18n` (next-intl underneath for Next) | Decided |

## 4 · Desktop & mobile

| Concern | Decision | Status |
| --- | --- | --- |
| Desktop | **Tauri** — Rust backend, shares crates with firmware and geometry | Decided |
| Mobile | **Tauri** — same shell as desktop. Native Rust on the phone, no webview-WASM | Decided |
| Mobile + desktop BLE | **`tauri-plugin-blec`** (btleplug-based). iOS is the least-proven path | Decided |
| Capacitor | **Dropped from the platform.** Available as an optional capability if a product needs a specific plugin | Decided |
| Browser | The one place the WASM boundary is unavoidable | Decided |

## 5 · Backend, edge & data

| Concern | Decision | Status |
| --- | --- | --- |
| Database + auth | **Supabase** (Postgres + Auth), declarative schema → generated migrations | Decided |
| Live/session state | **Cloudflare Durable Objects** (PartyServer/PartySocket, SQLite, Alarms) | Decided |
| Object storage / archive | **Cloudflare R2**, append-only NDJSON with `schema_version` | Decided |
| Analytics | **DuckDB over R2** | Decided |
| Transactional email | **Resend**, via `@fiducial/email` | Decided |
| CMS | **Payload** | Candidate — ROP-proven, not yet platform-generalized |
| Docs site | **VitePress** | Decided |
| Error tracking | — | **Gap** — deliberately undecided, inherited from ROP |
| Payments | — | **Gap** — decide at first paid product |

## 6 · Firmware & embedded

| Concern | Decision | Status |
| --- | --- | --- |
| Language | **Rust only.** No PlatformIO, no C++ | Decided |
| Framework | **Embassy** (async) — `embassy-stm32`, `embassy-rp` | Decided |
| MCUs | **STM32** · **RP2040 / RP235x** | Decided |
| Flash + debug | **probe-rs** (`runner = "probe-rs run --chip …"`) | Decided |
| Logging | **defmt** + `defmt-rtt` | Decided |
| On-target tests | **`defmt-test`** | Decided |
| Host-side firmware tests | Mocked HAL, `cargo test` | Decided |
| LoRa | **`lora-rs`** — `lora-phy`, `lora-modulation`, `lorawan-encoding`, `lorawan-device` | Decided |
| IP networking | **`embassy-net`** (smoltcp) | Decided |
| Custom link/network layer | **`fiducial-protocol`** — framing, fragmentation, addressing, above the radio | Decided |
| Hardware-in-the-loop rigs | — | Deferred until a board physically exists |

## 7 · Electronics (EDA)

| Concern | Decision | Status |
| --- | --- | --- |
| Substrate / source of truth | **KiCad** | Decided |
| Authoring layer | **atopile** — units, tolerances, assertions, constraint solving, part picking | Decided |
| Alternative under trial | **Zener** (Diode Computers) — Starlark, Rust compiler, Claude Code skill | Candidate → board two |
| Outputs | **`kicad-cli` jobsets** → Gerbers, drill, BOM, pick-and-place, PDF, STEP, **GLB** | Decided |
| Board data extraction | `kicad-cli` + direct `.kicad_pcb` S-expression parsing (Edge.Cuts, mounting holes, 3D-model Z-offsets) | Decided |
| ECAD↔MCAD round-trip | **KiCad StepUp** (FreeCAD) if a board edge ever needs pushing back | Candidate |
| Fab / assembly | JLCPCB, orderable from CI via atopile | Decided |

## 8 · Mechanical, CAD & 3D

| Concern | Decision | Status |
| --- | --- | --- |
| Generated geometry | **build123d** (Python / OCCT) — B-rep, exports **STEP · STL · glTF · 3MF**, exact mass properties | Decided |
| Interactive CAD | **Fusion 360 stays yours.** Its API is GUI-bound and cannot run headless, so it is not a pipeline executor. **STEP is the interchange** between hand-designed and generated parts | Decided |
| `no_std`-shareable geometry | **`csgrs`** — only for simple solids that must be reachable from firmware or WASM | Candidate — build only if a real need appears |
| Parametric definitions | `fiducial-geometry` (`no_std`) — parameters and constraints live in L0; heavy kernel work is a pipeline | Decided |
| Interchange formats | **STEP** (machining/injection) · **STL** (printing) · **glTF/GLB** (web, Blender) | Decided |
| FEA / thermal / CFD / structural sizing / materials data / cert evidence | — | **Gap** — CalculiX, Elmer, OpenFOAM are candidates. Heavy; decide on real need |

## 9 · Manufacturing

| Concern | Decision | Status |
| --- | --- | --- |
| Slicing | **OrcaSlicer** or **Bambu Studio** CLI — identical flags, headless in Docker. Output is `.gcode.3mf` | Decided |
| Print profiles | Declared facts (`machine.json`, `process.json`, `filament.json`) under `product/` | Decided |
| Derived print facts | Print time, material mass, cost — outputs of the slice pipeline, feeding assertions and BOM | Decided |
| Process tolerances | `tolerances.toml` per process: `fdm-0.4mm`, `sla`, `cnc`, `injection` | Decided |
| Calibration | Measure once per machine via a printed coupon; write the numbers back into `tolerances.toml` | Decided |

## 10 · Visualization & content

| Concern | Decision | Status |
| --- | --- | --- |
| Web 3D | **Three.js** — React Three Fiber (React) / **Threlte** (Svelte) | Decided |
| Asset format | **glTF/GLB** | Decided |
| Asset optimization | **`gltf-transform`** — Draco / meshopt | Candidate → Phase 15 |
| Offline renders & animation | **Blender**, headless CLI, on tag only | Candidate → Phase 15 |
| Maps | **Mapbox GL** via `@fiducial/maps` | Decided (opt-in per product) |

## 11 · Simulation, science & ML

| Concern | Decision | Status |
| --- | --- | --- |
| Numerical / physics | **Rust** — `fiducial-sim` + `rayon`; WASM single-threaded by default | Decided |
| Dataframes | **Polars** | Candidate |
| ML training | PyTorch via `uv` (pragmatic) vs `candle`/`burn` (Rust) | **Gap** — decide on real need |
| Structure | Any of these is **a pipeline** (§3 of the spec). The core absorbs them without change | Decided |

## 12 · Quality

| Concern | Decision | Status |
| --- | --- | --- |
| JS/TS lint + format | **Biome** — one tool, one CI-blocking gate | Decided |
| Rust lint + format | **rustfmt** + **clippy** (`-D warnings`) | Decided |
| Unit tests | **Vitest** (TS) · `cargo test` (Rust) | Decided |
| Integration tests | Real local Postgres, no browser, no mocked DB | Decided |
| E2E | **Playwright** — real backend, never mocked | Decided |
| Rust API breakage | **`cargo-semver-checks`** | Decided |
| Derive correctness | `fid derive --check` — stale artifacts and violated assertions fail CI | Decided |
| CI | **GitHub Actions** | Decided |
| CI cost policy | Text pipelines every PR; KiCad / Blender / slicer containers **on tag only** | Decided |

## 13 · Release & distribution

| Concern | Decision | Status |
| --- | --- | --- |
| JS versioning | **Changesets** | Decided |
| Rust versioning | **release-plz** — release PRs, semver-checks, workspace-aware | Decided |
| Registries | crates.io + JS registry, **public** | Decided |
| Names | `fiducial`, `fiducial-*`, `@fiducial/*` — **all verified free 2026-09-06**, claim in Phase 0 | Decided |
| Binary artifacts | GitHub Releases on tag + local build cache. Never committed | Decided |
| Text artifacts | Committed, diffable, `--check`ed | Decided |
| Web deploy | **Vercel** (Next SSR) · **Cloudflare Workers** (static/PWA, DO) | Decided |
| Schema deploy | Supabase GitHub integration. **Never `db push` by hand** | Decided |

## 14 · Agent tooling

| Concern | Decision | Status |
| --- | --- | --- |
| Distribution | **Claude Code plugin** from a marketplace in the platform repo; six lines in `.claude/settings.json` | Decided |
| Guardrails | `PreToolUse` guard, rules read from `fiducial.toml`. **Must be shell-aware** — the ROP version false-positives on quoted strings | Decided |
| Doc fallback | **Context7** for any library without its own MCP server or skill | Decided |
| Known MCP servers | Supabase · shadcn · Chrome DevTools · Playwright · GitHub · next-devtools | Decided |
| Session capture | Transcripts exported to `docs/sessions/`, opt-in per repo — keeps "repo is the database" true | Decided |

## 15 · Security, secrets & IP

| Concern | Decision | Status |
| --- | --- | --- |
| Secrets at rest | **`age` / SOPS**-encrypted env committed per repo; one key in a password manager. No new vendor | Decided |
| Secrets in CI | GitHub Actions secrets, pushed by scripts reading local env | Decided |
| Public tier licence | **MIT** — chosen partly because Apache-2.0's express patent grant works against retaining patent rights | Decided |
| Private tier | Product repos, domain logic, novel protocols, hardware designs — never published | Decided |
| Patent hygiene | **File a provisional before any public disclosure.** The EPO and most jurisdictions allow no grace period | Decided |
| Contributions | CLA required before accepting any, or dual-licensing later becomes impossible | Decided |
| Trademark | Handled separately from code licence | Decided |

---

## 16 · Extension & optimization

| Concern | Decision | Status |
| --- | --- | --- |
| Extension unit | **Capability** — fact schemas · pipelines · code · templates · guard rules · migrations · **mandatory `SKILL.md`** · docs · conformance tests | Decided |
| No privileged domains | Everything Fiducial ships is itself a capability. Electronics occupies the same slot as billing | Decided |
| Agent knowledge | Every capability ships a skill; the Claude Code plugin **aggregates skills of enabled capabilities** | Decided |
| Third-party tools | A capability may be only a skill + guard rules + a wrapper. This is the supported way to bring Blender, Stripe, a fab house or an instrument under the same discipline | Decided |
| Capability sources | Platform monorepo (first-party) or any git repo (third-party/private) | Decided |
| Derive graph shape | **DAG, permanently.** One declared design point. Coupled implicit solving is delegated | Decided |
| Design-space search | **OpenMDAO**, as an optimizer capability via `ExternalCodeComp` | Candidate — build on real need |
| Optimizer boundary | Cheap/analytic components inside the loop; expensive pipelines (KiCad, OCCT, slicer) only at the chosen point | Decided |
| Python ↔ L0 bridge | **PyO3** — OpenMDAO, build123d, science and ML share the same `Quantity` types and physics | Decided |
| Pipeline invocation | Library API **and** CLI, so an optimizer can drive derivation | Decided |
| Aerospace scope | Substrate for conceptual/preliminary design and the engineering-data backbone. **Not** a certification toolchain | Decided |
| Physics solvers for an optimizer to call | See §8 — the FEA/CFD gap is owned there, not duplicated here | Gap (§8) |

## 17 · Cost & frugality

| Concern | Decision | Status |
| --- | --- | --- |
| Cost posture | **Ceiling declared first as a fact; assertion fails until met** (§3.5). Cost is never merely reported | Decided |
| BOM cost | Derived — atopile part selection + JLCPCB pricing, per unit and at volume | Decided |
| Material / machine time | Derived — the slice pipeline already emits mass, time and consumables | Decided |
| Process ladder | Cheapest viable rung: FDM → SLA → CNC → injection; 2-layer → 4-layer; JLCPCB basic → extended. Escalation is a recorded decision | Decided |
| Tool selection | Cost is a selection criterion, not an afterthought: KiCad over Altium, FOSS throughout, free registries, no vendor priced on your success | Decided |
| Cloud cost | Derived per environment from vendor APIs | Candidate — build at second product |
| **Agent context cost** | Tracked by `fid doctor`; **every capability declares a context budget** for its skill | Decided |
| Model policy | Declared fact, not habit: strong model plans, cheap model executes; switch point defined as an event | Decided |
| Session hygiene | Long contexts cost more even when cached — clear between tasks, keep heavy skills scoped to the phase that needs them | Decided |

| AI-assisted authoring | Agents author **facts, decisions and pipeline code — never artifacts** (§3.6). build123d MCP; atopile and Zener Claude skills | Decided |
| Portfolio operations | **Portfolio manifest** — `fid doctor --portfolio`, `fid upgrade --portfolio` fan out across product repos (§3.7) | Decided |
| Portfolio secrets | One `age`/SOPS key per portfolio, not per repo | Decided |

| Firmware OTA | **`embassy-boot`** — A/B partitions, power-fail-safe swap, trial boot, auto-rollback, `ed25519` signature verification | Decided |
| Firmware update transport | **Declared per device** — BLE / Wi-Fi / USB. **Not LoRa**: impractical for a full image | Decided |
| Desktop updates | `tauri-plugin-updater`, signed binaries. Tauri compiles web assets into the binary — no web-only OTA natively | Decided |
| Mobile updates | Store release + **Capawesome Live Update** — rollback, staged rollout, delta | Decided |
| PWA updates | Service-worker update with an explicit `skipWaiting` policy | Decided |
| Version skew | **Protocol version is a declared fact; every artifact embeds it; compatibility is an assertion** (§12.1) | Decided |
| Release | `fid release` — one tag, every platform artifact, one protocol version, one provenance hash | Decided |
| Sync | **Idempotency by default.** Offline queue replays against the original action timestamp | Decided |
| Rust adoption | **Opt-in per product** via `fiducial.toml`; a pure-web product uses none (§4.1) | Decided |
| Anti-lock-in | **Commit to contracts, not tools.** One contract per domain; tools are swappable capabilities (§3.9) | Decided |
| Minimum capability | **One file — `SKILL.md`.** Manifest derived, everything else optional | Decided |


---

## Score

**141 rows. 122 Decided · 12 Candidate (each with a named phase) · 4 Gap · 2 Deferred.**

**The gaps, in full:** error tracking · payments · ML framework · FEA/thermal/CFD (which also
covers structural sizing, materials data and certification evidence).
**Deferred:** Ruby/Rails · hardware-in-the-loop rigs.

**Nothing on that list blocks Phase 0 through Phase 8.**

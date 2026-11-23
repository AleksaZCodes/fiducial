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
| Node | **22.18 or newer**, pinned by `.nvmrc` and `engines`. 22.18 is the floor because native TypeScript stripping and `module.registerHooks` are what let the Payload CLI run without its bundled tsx (`2026-11-23-analytics-is-a-capability-consent-is-opt-in`, D26); Node 20 went end-of-life in April 2026 | Decided — 2026-11-23, raised from 20 |
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
| Content-first framework | **Astro** (islands, zero JS by default) | Candidate — roadmap: *A stack chosen per app* |
| React, routes and loaders | **React Router** (framework mode) | Candidate — roadmap: *A stack chosen per app* |
| React, full-stack typed | **TanStack Start** | Candidate — roadmap: *A stack chosen per app* |
| Static / PWA | Next.js `output: "export"` on Cloudflare Workers Static Assets | Decided |
| Styling | **Tailwind v4**, CSS-first (`@theme`, no config file) | Decided |
| Design tokens | `@fiducial/tokens` — OKLCH palette as CSS custom properties | Decided |
| React primitives | **Base UI** via shadcn conventions | Decided |
| Svelte primitives | **shadcn-svelte / Melt** | Candidate → Phase 10 |
| Component distribution | Package vs shadcn-style copy-in registry | Candidate → Phase 10 |
| Forms | react-hook-form + Zod (React) | Decided |
| i18n | `@fiducial/i18n` (next-intl underneath for Next) | Decided |
| A client site that **sells** | **Next.js + Payload** — one app, admin included (`2026-11-22-payload-is-the-cms-and-commerce-capability`) | Decided |
| A client site that is **content only** | **Astro**, with the git-based editor. An Astro storefront over the Payload API stays possible and changes no data model | Decided |

## 4 · Desktop & mobile

| Concern | Decision | Status |
| --- | --- | --- |
| Desktop | **Tauri** — Rust backend, shares crates with firmware and geometry | Decided |
| Mobile | **Tauri** — same shell as desktop. Native Rust on the phone, no webview-WASM | Decided |
| Mobile + desktop BLE | **`tauri-plugin-blec`** (btleplug-based). iOS is the least-proven path | Decided |
| Mobile until Tauri mobile ships | **Responsive web app installed as a PWA** — one site on every screen size | Decided |
| Capacitor | **Optional shell**, chosen per app when a product wants a thin native wrapper around its web app and a specific plugin (BLE, geolocation, background tasks). Proven on a product; a `shell` capability is planned. Tauri stays the default | Decided |
| Electron | **Optional shell**, only where Tauri's webview differences or a Node-only dependency make it the cheaper choice. Planned with the other shells | Candidate |
| Browser | The one place the WASM boundary is unavoidable | Decided |

## 5 · Backend, edge & data

| Concern | Decision | Status |
| --- | --- | --- |
| Database + auth | **Supabase** (Postgres + Auth), declarative schema → generated migrations | Decided |
| Live/session state | **Cloudflare Durable Objects** (PartyServer/PartySocket, SQLite, Alarms) | Decided |
| Object storage / archive | **Cloudflare R2**, append-only NDJSON with `schema_version` | Decided |
| Analytics over **high-volume telemetry** | **DuckDB over R2** — device readings and archived NDJSON. Clarified 2026-11-22: it is *not* the answer for a client shop's dozens of orders a month, and 2026-11-23 it is not the `analytics` contract either — that contract's first-party provider writes site events to D1 | Decided |
| Business metrics for a shop | **The commerce data itself** — one timestamp per order status, queried over `orders` and `quotes`. No event store: a second source of truth for "is this paid" is worse than a slower query | Decided |
| Traffic analytics | **One `analytics` contract, many providers at once** — `track(event)` / `pageview(ctx)`, declared as a list in `[analytics] providers` because `[adapters]` is one vendor per contract. Default on and consent-free: **first-party** (an events table in D1 behind `/api/e`, nothing written to the device, visitor hash = HMAC(declared secret, UTC date, IP, user agent), raw IP never stored) and **Cloudflare Web Analytics** (cookieless beacon). Off by default: **PostHog** (`cookieless_mode: "always"`, EU host) and **Meta Pixel + Conversions API**, which `requiresConsent` and cannot be enabled without the `consent` capability. Spec: `2026-11-23-analytics-is-a-capability-consent-is-opt-in` | Decided — capability not built yet |
| Consent | **Its own opt-in capability, off by default.** The default template sets no non-essential cookies and loads no third-party scripts, so it ships no banner: strictly necessary storage is the cart in local/session storage and the staff auth cookie, and embedded maps and videos are click-to-load. Installing `consent` adds the banner and the footer preferences link, and is what makes a `requiresConsent` provider legal to enable | Decided — 2026-11-23, supersedes the `legal` capability's banner |
| First-party analytics retention | **13 months (`[analytics] retention_days = 396`), deleted by a Cron Trigger.** One declaration derives the job and the sentence in the privacy notice | Decided — designed 2026-11-23, not built |
| Transactional email | **Resend**, via `@fiducial/email` | Decided |
| CMS where the content is a **database** | **Payload 3** on Workers: D1 through `@payloadcms/db-d1-sqlite`, media on R2 through `@payloadcms/storage-r2`, modelled on the official `with-cloudflare-d1` template and deployed through this repo's `nextjs` shape. **Paid Workers plan** (CPU time; the bundle limit is 64 MiB uncompressed since the compressed limit was removed in September 2026 — carried from the decision brief, **not re-verified** against Cloudflare's docs). Spec: `2026-11-22-payload-is-the-cms-and-commerce-capability` | Decided — capability not built yet |
| Content editor (git-based) | **Keystatic** (default) and **TinaCMS**, both derived from `content.toml`, `messages/` and `settings.json`; **Pages CMS** (hosted, edits through GitHub; `.pages.yml` derived and held to its own schema). Decap was removed in 0.15.0 | Decided |
| Backend framework | **Hono** (Workers-native; Node for a self-hosted target) | Candidate — roadmap: *A stack chosen per app* |
| ORM | **Drizzle** over the `database` contract (D1, Postgres, SQLite) | Candidate — roadmap: *A stack chosen per app* |
| Docs site | **VitePress** | Decided |
| Self-hosted server | **Rust `axum` + `tokio`**, `sqlx` — a recorded exception to TypeScript server glue (`2026-11-01-a-self-hosted-target`) | Decided |
| Self-hosted database | **SQLite (WAL)** — raw readings 30 days, hourly roll-ups kept | Decided |
| Error tracking | — | **Gap** — deliberately undecided, inherited from ROP |
| Staff auth in a client app | **Payload's built-in auth** — staff only, no public sign-up. Access control calls `can()` from `@fiducial/identity`: owner → `owner`, staff → `member`, an agency API key → a `service` principal at `viewer`, all on resource `platform`, so the identity model is unchanged and no grants table exists | Decided |
| Customer accounts / **Better Auth** | — | Deferred — no client product has customers who log in |
| Commerce model | **The platform's own Payload collections** — products, variants, orders, quotes, one line-item model, prices in minor units. `@payloadcms/plugin-ecommerce` was evaluated 2026-11-22 and **not adopted**: Beta, a required `customers` collection, and the D1 adapter's no-op transactions with an open data-loss report on array updates (payload#15219, #14766) | Decided |
| Card payments | — | **Gap** — unchanged. The first paid product is confirmed by phone and paid on delivery or by bank transfer, and an admin action marks it paid. No payment adapter is written, not even a stub |
| Nightly database backup | **D1 export to R2 on a Cron Trigger**, 30 days kept, restore documented alongside D1 point-in-time recovery | Decided — designed 2026-11-22, not built |

## 6 · Firmware & embedded

| Concern | Decision | Status |
| --- | --- | --- |
| Language | **Rust only.** No PlatformIO, no C++ | Decided |
| Framework | **Embassy** (async) — `embassy-stm32`, `embassy-rp` | Decided |
| MCUs | **STM32** · **RP2040 / RP235x** · **ESP32-C6** (`esp-hal` + Embassy + `esp-radio` + `esp-rtos`) | Decided |
| Flash + debug | **probe-rs** (`runner = "probe-rs run --chip …"`) | Decided |
| Logging | **defmt** + `defmt-rtt` | Decided |
| On-target tests | **`defmt-test`** | Decided |
| Host-side firmware tests | Mocked HAL, `cargo test` | Decided |
| LoRa | **`lora-rs`** — `lora-phy`, `lora-modulation`, `lorawan-encoding`, `lorawan-device` | Decided |
| IP networking | **`embassy-net`** (smoltcp) | Decided |
| Custom link/network layer | **`fiducial-protocol`** — framing (`MAGIC · LEN16 · payload · CRC-32`), above the link | Decided |
| IoT transport | **MQTT 3.1.1 over TLS**, broker **Mosquitto**; per-device credentials, ACL derived from `can()` | Decided |
| Fieldbus to vendor sensors | **Modbus RTU over RS-485** — `fiducial-protocol::modbus`, tested over a virtual serial port | Decided — built |
| ESP firmware OTA | `esp-bootloader-esp-idf` partitions + `fiducial-ota` (`embassy-boot` has no ESP backend) | Candidate → first product that needs it |
| CAN / CAN-FD | ISO 15765-2 (ISO-TP) segmentation in `fiducial-protocol::segment`, frames in `fiducial-protocol::can`; FDCAN on MCUs, SocketCAN on Linux | Decided — the protocol is built and tested over an in-memory bus; no controller driver yet |
| Linux real-time node runtime | `PREEMPT_RT` (mainline since 6.12) as the kernel; `fiducial-rt` applies `SCHED_FIFO`, `mlockall`, CPU pinning and stack prefault from the node's `[node.rt]` declaration | Decided — built; hard latency still needs a `PREEMPT_RT` kernel and a measurement on it |
| Time-triggered / TSN Ethernet | — | **Gap** — no product needs one |
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
| Local inference | **`ollama`** (llama.cpp, OpenAI-compatible endpoint) behind the `ai` contract | Decided |
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
| Self-hosted deploy | **Docker Compose** on a Linux host (arm64 or x86_64), **Caddy** for TLS — derived like `wrangler.toml` | Decided |
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
| Agent knowledge | Every capability ships an Agent Skill (`.agents/skills/`, linked per agent); vendor skills a capability names are installed by the open `skills` CLI and pinned in `skills-lock.json` | Decided |
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

**170 rows. 143 Decided · 18 Candidate (each with a named phase) · 6 rows marked Gap · 3 Deferred.**

Recounted 2026-11-23 by reading the tables rather than by editing the previous
line, which said *167 · 140 · 18 · 6 · 3* for the tables as they stood before
the `analytics`, `consent`, retention and Node rows — a hand-maintained total is
a second declaration of the rows above it. The six Gap rows are **five
distinct gaps**: §16's physics-solver row points at §8's rather than owning one.

**The gaps, in full:** error tracking · **card payments** · ML framework · FEA/thermal/CFD
(which also covers structural sizing, materials data and certification evidence) ·
time-triggered / TSN Ethernet.
**Deferred:** Ruby/Rails · hardware-in-the-loop rigs · customer accounts / Better Auth.

**Nothing on that list blocks Phase 0 through Phase 8.**

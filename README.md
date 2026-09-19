# Fiducial

> **Declare each fact once. Derive every artifact from it.**

Fiducial is a cross-domain build system for products that span **web, firmware,
electronics, mechanical, simulation and content** — built at the quality and pace
that normally needs a team, by one person working with agents.

Not by working harder at each domain. By deleting the work that exists only
because the domains don't talk to each other.

---

## The problem it removes

In a cross-domain product the same fact gets written down many times:

- a **pin assignment** — in the schematic, the firmware, the test rig, the docs
- a **protocol message** — in the firmware, and again in the web client
- a **board dimension** — in the PCB tool, the enclosure model, the marketing render

Every duplicate is a place where reality drifts from itself. Drift is discovered
late, in the field, expensively.

So: **one declaration, many derivations.** The declaration is typed,
machine-readable and lives in git. Everything downstream is generated, and
generated artifacts are never hand-edited.

## The shape of the system

Three ideas stacked, and each one exists to make the one above it cheap.

```
                        ┌───────────────────────────────┐
   DECLARE              │  board.interface.json         │   typed facts,
   a fact, once         │  fiducial.toml  ·  messages/  │   inert, in git
                        └───────────────┬───────────────┘
                                        │
                                  fid derive          ← gated by --check
                                        │
        ┌───────────────┬───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼               ▼
   DERIVE          enclosure       TypeScript      wrangler.toml    migrations
   every artifact  STL · GLB       types           (deploy cfg)     SQL schema
        │
        └── never hand-edited · hashed into fiducial.lock · stale ⇒ CI fails

        firmware · desktop · browser · edge · CLI     ← many runtimes
                          ╲   │   ╱
   RUN IT ANYWHERE  fiducial-protocol                  ← one waist
                          ╱   │   ╲
        USB serial · Web Serial · WebUSB · BLE · LoRa  ← many transports
```

Logic lives as far down as it can. The `no_std` Rust spine compiles for the host,
`wasm32`, and two embedded targets **on every commit** — so a behaviour written
once runs in a browser, on a desktop, at the edge, and on a microcontroller, and
outlives every framework above it.

## A worked example

```sh
cargo install fiducial-cli

fid new my-product && cd my-product
fid add eda                # board pipeline + enclosure generation
fid derive                 # run every pipeline, hash every output
fid dash                   # roadmap, decisions, CI, pipelines, freshness
```

Declare the board once, in `board/board.interface.json`:

```json
{
  "outline": { "width_mm": 100.0, "height_mm": 60.0, "tolerance": "fdm" },
  "connectors": [
    { "id": "J1", "type": "usb-c", "mount": { "side": "south", "offset_mm": 20.0 } }
  ]
}
```

Then `fid derive` produces — with nobody modelling anything —

- a **gasket-sealed enclosure**, with the USB-C opening already punched, sized
  from the connector family and the printing process
- a printable **STL** and a web-ready **GLB**
- **TypeScript types** for the same board, for the app that talks to it

Change `width_mm` to `120.0` and every one of those moves. Until they do,
`fid derive --check` says so and exits non-zero:

<!-- capture: fid-derive-check-stale.txt -->

```text
$ fid derive --check
✗ fid derive --check failed:
  board/board.interface.json: stale (lock:9d241d22 file:867aeb34) — run `fid derive`
Error: stale artifacts detected
```

<!-- /capture -->

That is the whole contract. `fid derive --check` runs in CI, so an artifact that
did not follow its declaration fails the build instead of shipping.

## What you can add to it

Nothing above is special-cased into the CLI. Each of these is a **capability** —
a named, versioned extension that contributes declarations, pipelines, guard
rules and a skill for agents. `fid new` generates none of them; you add what the
product actually needs.

<!-- fid:begin capabilities -->
| Capability | Contributes | Install |
|---|---|---|
| `adapters` | declares `adapters`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability adapters` |
| `brand` | declares `brand`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability brand` |
| `deploy` | declares `deploy`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability deploy` |
| `design` | declares `design-system.md`; 1 pipeline(s); 4 template file(s) | `fid add capability design` |
| `eda` | declares `board/board.interface.json`; 2 pipeline(s); 1 template file(s) | `fid add capability eda` |
| `firmware-rp2040` | 10 template file(s); guard rules | `fid add capability firmware-rp2040` |
| `firmware-stm32` | 9 template file(s); guard rules | `fid add capability firmware-stm32` |
| `i18n` | declares `i18n`, `messages/en.json`, `messages/sr.json`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability i18n` |
| `identity` | declares `identity`; 1 pipeline(s); seeds a `fiducial.toml` block | `fid add capability identity` |
| `legal` | declares `legal`; 1 pipeline(s); 2 template file(s); seeds a `fiducial.toml` block | `fid add capability legal` |
| `migrations` | declares `migrations`; 1 pipeline(s) | `fid add capability migrations` |
| `realtime` | a skill | `fid add capability realtime` |
| `tauri` | 5 template file(s); guard rules | `fid add capability tauri` |
| `web-next` | 8 template file(s); guard rules | `fid add capability web-next` |
| `web-svelte` | 8 template file(s); guard rules | `fid add capability web-svelte` |
| `worker-cloudflare` | 1 template file(s); guard rules | `fid add capability worker-cloudflare` |
<!-- fid:end capabilities -->

<!-- fid:describes crates/fiducial-cli/src/adapter.rs#pub static CONTRACTS -->

Vendors sit behind **adapter contracts**, chosen per product in `[adapters]`.
Every contract ships with `none` — a real, working implementation, not a
placeholder, which is what makes it cost nothing to wire in from the first
commit. A name under *Planned* cannot be selected and fails with a message
saying so: a selectable name with nothing behind it is a promise the platform
does not keep.

Two of those `none`s **fail rather than succeed silently**: `auth` and `ai`.
The difference is whether the caller reads a result. A no-op send or enqueue is
indistinguishable from the real thing at the call site — the caller wanted an
effect elsewhere. A session and a completion *are* the result, so returning a
fabricated one turns "no vendor selected" into a logged-in stranger or a blank
answer in the UI, and both get debugged as bugs somewhere else. They still cost
nothing to wire in: constructing them is free, and nothing fails until
something actually asks.

<!-- fid:end-describes -->

<!-- fid:begin adapters -->
| Contract | For | Selectable today | Planned |
|---|---|---|---|
| `database` | Relational storage: queries, migrations, transactions | `none`, `d1`, `supabase` | `neon`, `postgres` |
| `storage` | Object storage: put, get, signed URLs | `none`, `r2`, `supabase-storage` | `s3` |
| `deploy` | Where the product ships and how a release is promoted | `none`, `cloudflare` | `vercel`, `fly` |
| `email` | Transactional email: send, template, verify a domain | `none`, `resend` | `ses`, `cloudflare-email` |
| `newsletter` | Subscriber list management: subscribe, unsubscribe, status | `none`, `resend` | — |
| `errors` | Error tracking and diagnostics | `none` | `sentry`, `workers-analytics` |
| `botProtection` | Bot / abuse challenge verification | `none`, `turnstile` | `recaptcha`, `hcaptcha` |
| `queue` | Asynchronous job/message queue (producer side) | `none`, `cloudflare-queues` | `sqs` |
| `ai` | Language-model calls: chat, streaming, tool use | `none`, `openrouter` | `workers-ai`, `anthropic`, `openai` |
| `auth` | Users and authentication: sign-up, sign-in, sessions | `none`, `supabase` | `clerk`, `auth.js` |
<!-- fid:end adapters -->

Both tables are generated from the registries that enforce them, by
`fid context`, and CI fails if they drift from the code.

## How you extend it

### A capability is a directory

There is no plugin API to learn and no CLI release to wait for. The **layout is
the manifest** — `fid` derives the capability from the directory, and a
third-party capability goes through the same derivation, the same conformance
checks and the same lock entry as a built-in.

```
my-capability/
├── SKILL.md              ← the only required file: how an agent uses this
├── capability.toml       ← optional: description, guard rules, [config] seed,
│                           declared directories, required adapter contracts
├── declarations/         ← typed facts, installed at the path below this dir
│   └── messages/en.json
├── pipelines/
│   └── i18n.toml         ← name, executor, args, outputs
└── anything-else         ← a plain template file, copied in, gated by nothing
```

Scaffold one, then install it from anywhere — a directory, or a git repository
pinned in `fiducial.lock` at the commit it resolved to:

```sh
fid capability new my-sensor                    # scaffold the directory above

fid add capability my-sensor --from ./capabilities/my-sensor
fid add capability stripe    --from git:https://github.com/acme/fid-stripe#v1.2.0
fid capability check                            # conformance, once installed
```

One file is a complete capability. The failure mode for an extension system is
ceremony, so `capability.toml` is optional and a capability without one takes
its description from the first line of prose in its own skill.

### The four kinds, and why the difference is not cosmetic

<!-- fid:describes crates/fiducial-cli/src/capability/manifest.rs#pub fn derive -->

| Kind | Is | Example |
|---|---|---|
| **Declaration** | a typed fact, written once, inert — a file, a `fiducial.toml` block, or a directory the product fills | `board/board.interface.json`, the `[i18n]` block, `migrations/` |
| **Pipeline** | reads declarations, produces artifacts, **gated by `fid derive --check`** | `pipelines/eda.toml` |
| **Adapter** | a swappable vendor behind a fixed contract | `storage = "r2"` |
| **Tool** | an external command the capability's work needs on PATH — declared, never installed | `requires_tools = ["wrangler"]` |
| **Template** | a plain file copied in, belonging to no pipeline | `apps/worker/wrangler.toml` |

<!-- fid:end-describes -->

The test for a declaration: *could two different pipelines read this and both be
correct?* If yes it is a declaration; if it is one tool's config file it is a
template.

Filing one as another is not a style mistake. A pipeline outside `pipelines/` is
installed and never runs; a `pipelines/` file listed as a template is installed
and never gated. `fid capability check` rejects both.

### A pipeline is four lines

```toml
name     = "enclosure"
executor = "fid-mesh"
args     = ["board/board.interface.json"]
outputs  = [
  "enclosure/case-base.stl",
  "enclosure/case-lid.stl",
  "enclosure/gasket.stl",
  "enclosure/case.glb",
]
```

`outputs` is what makes it gated: `fid derive` hashes each one into
`fiducial.lock`, and `fid derive --check` fails when a hash no longer matches
what the declaration implies.

<!-- fid:describes crates/fiducial-cli/src/commands/derive.rs#fn run_pipeline_command -->

Executors are `shell` and `cargo-test` — which need no platform change at all —
plus the in-process ones (`fid-validate`, `fid-mesh`, `fid-i18n`, `fid-brand`,
`fid-deploy`, `fid-identity`, `fid-adapters`, `fid-schema`, `fid-legal`,
`fid-design`). Reach for `shell` first; a new in-process executor is warranted
only when the work is genuinely a Rust library call rather than a tool
invocation.

`fid-design` is the clearest case of that line: it parses a declaration, runs
OKLCH-to-sRGB conversion and WCAG contrast arithmetic over the palette, and
fails the derive when a declared pair misses its minimum. Shelling out would
mean shipping a script and a language runtime to do arithmetic the binary
already can.

<!-- fid:end-describes -->

### Adding a vendor to a contract

Implement the contract in `crates/fiducial-adapters` (and its TypeScript mirror
in `packages/adapters`), then move the vendor from `candidates` to
`implementations` in `crates/fiducial-cli/src/adapter.rs`. The table above and
the error message a user sees both derive from that one move.

One place does need the new name: `REAL_VENDORS` in
`packages/adapters/src/generated-factory.test.js`, which is the list of vendor
selections CI actually compiles the generated factory for. A vendor missing
from it is generated and never typechecked, so the suite names itself in that
comment rather than leaving it to be discovered.

Deeper detail: [`docs/specs/2026-09-15-external-capabilities.md`](./docs/specs/2026-09-15-external-capabilities.md)
and [`docs/specs/2026-09-14-capability-taxonomy.md`](./docs/specs/2026-09-14-capability-taxonomy.md).

## Documentation

**New here?** [Start here](./docs/guides/start-here.md) explains what this is and
why it is shaped that way, then [Your first product](./docs/guides/first-product.md)
takes you from nothing to a generated enclosure and a CI gate in about twenty
minutes.

| Read | For |
|---|---|
| [**docs/guides/**](./docs/guides/) | Step-by-step guides for humans and agents |
| [`AGENTS.md`](./AGENTS.md) | Working here as an agent — layout, rules, build commands |
| [`MISSION.md`](./MISSION.md) | Why this exists. The tiebreaker for ambiguous decisions. |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | How the layers fit together. |
| [`STACK.md`](./STACK.md) | Every technology choice, enumerated. |
| [`ROADMAP.md`](./ROADMAP.md) | What is intended, in order — and so what is next. |
| [`SHIPPED.md`](./SHIPPED.md) | What was built, phase by phase. |
| [`docs/protocol/`](./docs/protocol/) | The wire specification + conformance vectors. |
| [`docs/specs/`](./docs/specs/) | Design decisions, append-only. |

Each crate and package carries its own README — start with
[`fiducial-cli`](./crates/fiducial-cli) if you want to build something, or
[`fiducial-core`](./crates/fiducial-core) if you want to see the spine.

## Repository

`firmware/` is a separate Cargo workspace (Embassy; RP2040 + STM32), and `docs/`
holds the specs, the protocol and the compatibility matrix. The two workspaces
this repository builds:

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
└── packages/             12 members
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
    ├── viewer3d-react           React component for rendering Fiducial board GLB files in a…
    └── wasm-bridge              Generated TypeScript types for the fiducial WASM boundary.
```
<!-- fid:end layout -->

## License

MIT — see [`LICENSE`](./LICENSE).

Product repos, domain logic, novel protocols and hardware designs are private;
see [`IP-POLICY.md`](./IP-POLICY.md).

Contributions are welcome and need a sign-off — see [`CLA.md`](./CLA.md). It
exists so the project keeps the option to relicense later: that option is
destroyed silently by the first contribution merged without one, and cannot be
recovered afterwards.

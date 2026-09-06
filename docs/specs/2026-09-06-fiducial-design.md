# Fiducial — Design

**Date:** 2026-09-06 · **Status:** awaiting review
**Mission:** `../../MISSION.md` (why) · **Stack:** `../../STACK.md` (what, finitely enumerated)
**Source material:** `/home/aleksa/dev/rop-reference/` — full-history copy of `ring-of-pursuit`
at `b16b226`. It is also the **pre-migration snapshot** the characterization suite compares
against (§6). Do not modify. Contains real `.env` secrets; never push it anywhere.

> **Name.** A *fiducial* is the reference mark on a PCB that a pick-and-place machine aligns
> every component against — the single declared reference from which all placement derives.
> Verified free on 2026-09-06: `fiducial` and every `fiducial-*` crate on crates.io, the
> `@fiducial` scope and bare `fiducial` on the JS registry. Claim in Phase 0.
> Supersedes `2026-09-06-platform-design.md`.

---

## 1. What this is

`MISSION.md` states the goal. This states the machine.

**This is not a hardware platform.** It is a system for building products that span whatever
domains they happen to span — web, business logic, content, electronics, mechanical, firmware,
science, simulation, ML. Hardware appears often in the examples below because it is where the
integration pain is most visible, not because it is privileged. Structurally it is not: every
domain, including electronics, is a **capability** (§3.3), and they all use the same mechanism.

Two products anchor the design, deliberately different in shape:

- **Ring of Pursuit** — a live outdoor game: web, realtime, Postgres, edge, content. No hardware
  at all. Already built, running real events, and the first commercial consumer (§6).
- **A LoRa walkie-talkie for hikers** — PCB, firmware, radio driver, a custom network stack with
  fragmentation and addressing, a Web Bluetooth client, an enclosure, a marketing site. Already
  built once, and its worst cost was **a network stack written twice, in C++ and JavaScript,
  that drifted.**

The second is the sharper illustration; the first is the one paying rent. A design that serves
only one of them is wrong.

**And a real product is every domain it touches at once, in sync** — not a sequence of domains
picked off one at a time. That is the constraint the whole system exists to satisfy.

---

## 2. Decisions on record

| Decision | Choice |
| --- | --- |
| Name | **Fiducial** · CLI `fiducial`, alias `fid` · crates `fiducial-*` · JS `@fiducial/*` |
| Reuse model | Platform repo + versioned packages |
| Registries | **Public** — crates.io and the JS registry. No auth on install, anywhere |
| Licensing | **Two tiers.** Public platform = **MIT**. Private products = never published (§14) |
| Business logic home | **Rust-first where it earns its place — opt-in per product** (§4.1). A pure-web product may use no Rust at all |
| UI sharing | **Tokens + headless only.** Components written per framework |
| Web frameworks | Next.js and SvelteKit first-class. Rails designed-for, not built |
| Desktop | **Tauri** — Rust backend, shares crates with firmware and geometry |
| Firmware execution | **Embassy async executor — not an RTOS.** RTIC is the documented escape hatch (§7.2) |
| Mobile | **Tauri** — same as desktop. Capacitor dropped from the platform; available as an optional capability (§7.1) |
| Firmware | **Rust only** — Embassy + probe-rs |
| Electronics | **KiCad is the substrate. atopile is the authoring layer.** Zener trialled on board two (§9) |
| Mechanical | **build123d** (Python/OCCT) generates geometry; **Fusion 360 stays the interactive CAD**; STEP is the interchange. No kernel written |
| Manufacturing | **OrcaSlicer / Bambu Studio CLI** headless; print profiles and process tolerances are declared facts |
| Artifacts | **Text committed, binaries as release artifacts** (§13) |
| Propagation | Codemods + template 3-way merge + `doctor` |
| Ring of Pursuit | **Migration is optional and on your schedule.** Nothing in Fiducial depends on it (§6) |
| Vendor lock-in | **Commit to contracts, not tools.** Every domain has one contract; tools are swappable capabilities (§3.4) |
| Release & OTA | One release, many artifacts, one protocol version; `embassy-boot` A/B + rollback for firmware (§12) |
| Growth discipline | **Rule of two** (§15) |

---

## 3. The core model

Everything in this system is one of **five primitives**. They are distinguished by *lifecycle
rule*, which is what makes them worth separating.

| Primitive | Is | Lifecycle rule |
| --- | --- | --- |
| **Fact** | A declared value — typed, united, toleranced, attributed | Authored by hand, exactly once. **Never generated** |
| **Decision** | A declared judgment with rationale and date | Appended, never edited. Superseded, never deleted |
| **Pipeline** | A pure transformation `(facts, artifacts) → artifacts` | Code. Declares its own inputs and outputs |
| **Artifact** | A derived output | Generated. **Never hand-edited** — guard-blocked |
| **Graph** | The DAG connecting the four above | Derived. Queryable by humans, agents, and the workbench |

**The universality test, and why the previous draft failed it.** Adding a new domain —
chemistry, acoustics, ML — must require *new fact types and new pipelines, and no change to the
core*. The earlier spec listed pipelines as named special cases (`eda`, `mech`, `types`,
`docs`), which is a list, and a list never generalizes. Defining the **pipeline contract**
instead makes every domain an implementation of one interface. A simulation is a pipeline whose
output is data; an ML training run is a pipeline whose output is a model; a Gerber export is a
pipeline whose output is a fab file. The core does not know the difference, and that is the
point.

### 3.1 The atom: a quantity with a unit and a tolerance

A physical fact is never a bare number.

```rust
Quantity<Length>  = 1.6 mm ± 10%
Quantity<Freq>    = 868 MHz ± 20 ppm
Quantity<Time>    = 12 h  (min)
```

Dimensions are checked at compile time (`uom` / `dimensioned`), so millimetres can never be
added to millihenries. `no_std`, so the same value is readable by firmware, the enclosure
generator, the web configurator, the simulation, and the spec sheet.

**Tolerance stack-up is the feature, and it is the same problem in every domain:**

| Domain | The stack | The question it answers |
| --- | --- | --- |
| Electronics | resistor ±1% · rail ±5% | Does the divider stay in spec? |
| Mechanical | board ±0.1 mm · wall ±0.15 mm · print shrink ±0.2 mm | **Does the lid close?** |
| Power | draw ±10% · cell capacity ±5% | Does it really last 12 h? |
| Marketing | all of the above | Which numbers may I publish? |

atopile solves this inside a schematic. Fiducial solves it across the product. **Assertions over
quantities** are the checkable form of "will it work":

```
assert enclosure.internal_height >= board.thickness + tallest_component.height + clearance
assert battery.capacity / system.draw_avg >= spec.battery_life_min
```

`fid derive --check` evaluates assertions with tolerances propagated. A violated assertion fails
CI. **This is the thing that makes "designed in the virtual world before it leaves the factory"
a mechanism rather than an aspiration.**

### 3.2 The pipeline contract

```
pipeline <name>
  inputs   facts[...]  artifacts[...]     ← declared, not discovered
  outputs  artifacts[...]
  run      <executor>
  check    hash(inputs) == recorded ⇒ up to date
```

Every artifact records the content hashes of every input and the commit that produced it. That
gives **provenance for free**: *which commit produced the Gerbers I sent to the fab, and which
firmware is on this device?* — answerable, not archaeology. It is also what makes reproducibility
real for science and compliance work later.

### 3.3 Capabilities — the one extension unit

**Everything that extends Fiducial is a Capability, including everything Fiducial ships with.**
Electronics is not built in; it is a first-party capability occupying exactly the same slot as
billing, chemistry, or an ML training loop. This is what keeps the system domain-neutral: there
is no privileged domain, because there is no mechanism a privileged domain could use.

A capability is a directory that may contribute any of:

| Contributes | Purpose |
| --- | --- |
| **Fact schemas** | What this capability lets you declare |
| **Pipelines** | What it derives, per the §3.2 contract |
| **Crates / packages** | Code, at whichever layer it belongs to (§4) |
| **Templates** | Scaffolding it adds to a product |
| **Guard rules** | What it forbids, read from `fiducial.toml` |
| **Migrations** | Codemods and template patches for its own upgrades |
| **`SKILL.md`** | **Mandatory.** How an agent uses it correctly |
| **Docs + conformance tests** | How a human uses it; proof it satisfies the contract |

**The hard rule: no `SKILL.md`, not a capability.** A capability shipping code without agent
instructions is one an agent must reverse-engineer every session, which is the opposite of
`MISSION.md` principle 4. The platform's Claude Code plugin **aggregates the skills of every
enabled capability**, so enabling one makes an agent immediately competent at it — with no
upgrade step, per §11.

A capability may be *nothing but* a skill plus guard rules plus a thin wrapper — that is the
supported way to bring a third-party tool (Blender, Stripe, a fab house, an instrument) under
the same discipline as first-party code.

**The minimum viable capability is one file.** This is a hard design constraint, not a nicety:
the failure mode for an extension system is ceremony, and a capability that needs a manifest, a
skill, tests, docs and migrations before it does anything is one nobody creates. So: **only
`SKILL.md` is required.** The manifest is derived from the directory, not hand-written. Everything
else is optional and added when the capability actually grows into it. A capability may legitimately
be a single skill file that teaches an agent to use a tool correctly — that is a complete,
supported capability.

```
fid capability new <name>      scaffold one, conformance tests included
fid capability check           schemas valid · pipelines declare I/O · skill present · docs present
fid add <capability>           install into this product; guard rules and skill activate
fid capability list            what is enabled here, and at what version
```

Capabilities are versioned and upgraded exactly like packages (§11). They resolve from the
platform monorepo (first-party), or any git repo (third-party or private) — the same source
model the Claude Code marketplace already supports.

**Domains are just sets of capabilities.** "Electronics" is `atopile` + `kicad` + `fab`;
"aerospace" would be `mdao` + `cfd` + `structures`. Adding a domain adds capabilities and
changes nothing in the core — which is the §3 universality test, now with a mechanism behind it.

### 3.4 Contracts, not tools — how to be opinionated without lock-in

Being "extremely opinionated for efficiency" and "free of vendor lock-in" sound opposed. They are
reconciled by being opinionated about **the wrong thing on purpose**:

> **Commit to the contract. Never commit to the tool that satisfies it.**

Each domain has exactly one contract, and it is the contract that is opinionated:

| Domain | The contract — fixed | The tool — swappable |
| --- | --- | --- |
| Electronics | `board.interface.json` (§8.1) | atopile · Zener · hand-drawn KiCad |
| Mechanical | STEP · STL · glTF + mass properties | build123d · csgrs · a Fusion export |
| Protocol | The `postcard` schema | any transport |
| Web | Tokens + headless logic | Next.js · SvelteKit · anything |
| Data | The Postgres schema | Supabase · any Postgres |
| Manufacturing | `.gcode.3mf` + print facts | Orca · Bambu · Prusa |

**So "I want to use Zener instead" is a one-line change**, not a migration: swap
`eda-atopile` for `eda-zener` in `fiducial.toml`. Both capabilities emit the same
`board.interface.json`, so the enclosure generator, the renders, the BOM and the spec sheet
neither know nor care. That is the concrete payoff of the capability model, and it is the reason
picking atopile first costs you nothing.

The same logic protects every other row. A tool that cannot be made to satisfy the contract is a
tool this system will not adopt — which is the opinionated half, doing real work.

### 3.5 External optimizers, and why the graph stays a DAG

The derive graph is a **DAG, deliberately and permanently.** It evaluates *one declared design
point*. Searching a design space, and solving *coupled implicit systems* where A depends on B
depends on A, are different jobs — delegated to an optimizer capability, never reimplemented here.

**OpenMDAO is the reference integration**, and the structural fit is close enough to be worth
naming: its components declare inputs and outputs (the §3.2 pipeline contract), its models are a
component graph, its variables carry **units** (§3.1), and its design variables, constraints and
objectives map onto facts and assertions. `ExternalCodeComp` exists precisely to wrap an external
executable, and `fid derive` is one.

The loop:

```
optimizer proposes a design point  →  written as facts  →  fid derive  →
artifacts + derived quantities (mass, cost, print time, margin)  →  objective + constraints back
                                          ↓ on convergence
              the winning design point is COMMITTED as declared facts,
        and CAD, firmware constants, BOM, spec sheet and marketing all re-derive
```

**Three constraints stated up front, because each is a trap:**

1. **Expensive pipelines must not sit inside the loop.** A KiCad export, an OCCT build or a slice
   takes seconds to minutes; gradient-based search wants thousands of evaluations. Cheap analytic
   or surrogate components go in the loop; expensive pipelines run at the chosen point.
2. **Fiducial pipelines are black boxes**, so an optimizer finite-differences them — costly and
   noisy. Components that want to participate properly should expose analytic partials.
3. **This is not aerospace-specific.** The same shape is unit-economics optimization for a
   business, hyperparameter search for a model, or mass-versus-cost for a part. Aerospace is
   simply where the tooling is most mature.

**Four decisions taken now because they are cheap now and expensive to retrofit:**

- Pipelines are callable as a **library API**, not only from the CLI, so an optimizer can drive them.
- Facts are **writable programmatically**, so an optimizer can propose a design point.
- **PyO3 bindings for L0**, so Python — OpenMDAO, build123d, science, ML — uses the same
  `Quantity` types and the same physics rather than a reimplementation.
- The DAG boundary above is **written down**, so nobody later tries to grow a solver into the
  derive runner.

**Honest scope for aerospace.** The architecture fits the parts that are usually hardest:
multi-domain coupling, units, tolerance stack-up, provenance and configuration control,
requirements traceability. It does **not** supply CFD, FEA, structural sizing, materials data, or
certification evidence — those are gaps, some of them heavy. Fiducial is a credible substrate for
conceptual and preliminary design and for the engineering-data backbone. It is not a
certification toolchain, and nothing here should be read as claiming otherwise.

### 3.6 Cost is a first-class derived fact

Cost is not reported at the end of a project. It is **declared as a ceiling at the start**, and
derived continuously — which makes it assertable (§3.1) and optimizable (§3.5).

**The inversion, following Prakash.** `spec.target_unit_cost` is a fact written before the design
exists. Its assertion fails from day one and keeps failing until the design meets it. That is the
point: a hard ceiling forces a different design, not a degraded one.

**Every cost is derivable from a pipeline that already exists:**

| Cost | Derived from |
| --- | --- |
| Bill of materials, per unit and at volume | atopile part selection + JLCPCB pricing (§9) |
| Material mass, machine time, consumables | The slice pipeline (§8) — already produces these |
| Board fabrication tier | Board area, layer count, minimum feature size from KiCad |
| Assembly | Placement count and part class (basic vs extended) |
| Cloud runtime | Vendor APIs, per environment |
| **Agent cost** | Resident context and tokens per session — see below |

```
assert bom.unit_cost_at_1k          <= spec.target_unit_cost
assert enclosure.material_mass      <= spec.material_budget
assert assembly.mass                <= spec.max_weight
```

**Cheapest viable process, declared as a preference ladder.** FDM → SLA → CNC → injection;
two-layer → four-layer; JLCPCB basic parts → extended. A pipeline selects the cheapest rung that
satisfies the declared requirements, and escalating a rung is a decision (§3) with a recorded
reason — not a default that quietly drifts upward.

**Tooling is chosen for cost as well as fit**, and `STACK.md` already reflects it: KiCad rather
than Altium, FOSS throughout, free registries, no vendor whose pricing scales with success. That
was not an accident and is now a stated rule.

**Agent cost is tracked like any other cost, and Ring of Pursuit already invented the metric.**
Its `STATUS.md` reports *"Resident context (`CLAUDE.md` + `AGENTS.md`): 4184 words ≈ 5.9k tokens,
paid every turn."* Generalized here:

- `fid doctor` reports resident context cost and flags growth.
- **Every capability declares a context budget** for its skill. The plugin aggregates skills
  (§3.3), so an unbounded capability set is an unbounded per-turn tax on every session.
- The model policy is a declared fact, not a habit: planning on the strong model, execution on
  the cheap one, with the switch point defined as an event rather than a feeling.

A system whose own operating cost is untracked has no standing to assert a cost ceiling on
anything else.

### 3.7 AI-assisted authoring — agents write declarations, never artifacts

"Generative" was asked for in two senses, and only one of them is safe.

**Generative parametric** — geometry produced by code from parameters — is §8. **Generative AI** —
an agent proposing an enclosure from the board and its components — is this section, and it has
exactly one rule:

> **An agent may author facts, decisions, and pipeline code. An agent may never author an
> artifact.**

An agent reads `board.interface.json` and writes **build123d source** or an `enclosure.rs` of
parameters. That source is reviewed, committed, and versioned like any other code; the STL, STEP
and GLB are still produced by the pipeline. What you get is *visible, editable geometry code* —
not an opaque mesh nobody can modify or explain.

Why the rule is absolute:

- **Provenance survives** (§3.2). An AI-emitted artifact has no derivable inputs, so its hash
  chain is broken and `--check` becomes meaningless.
- **The escape hatch survives** (`MISSION.md` principle 3). Generated code can be read, edited,
  and taken over by hand. Generated geometry cannot.
- **Review survives.** A diff of parameters is reviewable; a diff of a mesh is not.

This is already guard-enforced — artifacts are never hand-edited, and an agent is no exception.
It generalizes past geometry: an agent may draft an atopile schematic, a fact schema, a pipeline,
a test, or a brief. It may never emit Gerbers, a G-code file, or a generated type.

**This is also what "more declarative, less prompting" means concretely.** The durable object is
the *declaration*; prompting is how it gets drafted. A conversation that ends with committed facts
has produced something the system can derive from, check, optimize over, and re-derive in a year.
A conversation that ends with an artifact has produced something nobody can maintain.

Tooling: build123d has an MCP server, and atopile and Zener both ship Claude Code skills — so each
is a capability (§3.3) whose skill teaches the agent to author in the right layer.

### 3.8 Portfolio scale — many products, one system

The system is judged at *n* products, not one. What changes at scale, and the answer:

| At scale | Handled by |
| --- | --- |
| A platform release must reach 20 products | **A portfolio manifest** — a local file listing your product repos. `fid doctor --portfolio` reports which are behind; `fid upgrade --portfolio` fans out, one branch and PR per repo |
| Which product runs which capability at which version | `fid capability list` per repo, aggregated by the portfolio view and by workbench v1 (already multi-repo, §10) |
| Per-turn agent tax grows with capability count | Every capability declares a **context budget** (§3.6); `fid doctor` flags growth |
| CI cost multiplies by product count | The §14 policy — text pipelines per PR, heavy containers on tag — is what keeps this sublinear |
| Secret rotation across many repos | One `age`/SOPS key per portfolio, not per repo |
| A capability turns out wrong after wide adoption | Codemods (§11) are the migration path; the rule of two (§16) is what stops it spreading before it is proven |

**The limit that is real and stated:** propagation fans out linearly. Twenty products means twenty
upgrade PRs, each needing a green CI run and a glance. That is acceptable for a one-person
portfolio *because* the merge is usually trivial, and it is the direct reason 3-way merge and
codemods are Phase 4 rather than an afterthought — without them this number is where the system
would collapse.

### 3.9 Enforcement

`fid derive` runs pipelines. **`fid derive --check` fails CI on any stale artifact or violated
assertion.**

This is deliberately the exact shape of Ring of Pursuit's `status.mjs --check` and
`check-migrations.mjs`: a rule stated in prose, blocked at commit time by a hook, blocked at
agent time by the guard, backstopped in CI for the path that skips both. That triple is why
ROP's rules hold under deadline pressure, and it is the only reason to believe these will.

---

## 4. Two axes

The previous draft conflated these. They are orthogonal, and naming both is what lets an unknown
future domain slot in.

**Axis A — reach.** How far does this travel? Push behavior down; the lower it sits, the longer
it lives.

| Layer | Contents | Reaches |
| --- | --- | --- |
| **L0** Rust `no_std` | Facts, quantities, protocol, rules, geometry, math | Browser · Tauri · Workers · Node · STM32 · RP2040 · Ruby via `rb-sys` — **everything** |
| **L1** Tokens | CSS custom properties, Tailwind preset, 3D materials | Any web framework; also render pipelines |
| **L2** Headless TS | Transports, Supabase clients, WASM loading, offline queue | Any JS framework |
| **L3** Framework UI | Components | **One framework.** Deliberately thin |
| **L4** Repo system | CLI, guardrails, codemods, derive runner, CI, templates | Everything, language-agnostic |

**Axis B — domain.** Electronics · Mechanical · Firmware · Web · Edge · Content · Analysis. A
domain is nothing but *a set of fact schemas plus a set of pipelines*. That is the whole
definition, and it is why adding one is cheap.

### 4.1 Rust is opt-in per product, and here is when to opt in

Rust-first is a strong default, not a mandate. **A product declares in `fiducial.toml` whether it
uses the L0 spine**, and a product that does not still gets everything else — CLI, guardrails,
capabilities, propagation, tokens, test harness, CI, workbench, derive pipelines. Ring of Pursuit
could remain pure TypeScript forever and lose none of that.

The honest decision rule:

| Use Rust at L0 when | Stay in TypeScript when |
| --- | --- |
| The same logic must run in more than one runtime — especially if one is a microcontroller | It only ever runs in a browser or a Node/edge server |
| It is a protocol, a unit-bearing calculation, geometry, or a state machine | It is UI, routing, or server glue |
| Getting it wrong is expensive and the type system earns its keep | Iteration speed matters more than reach |

**The WASM boundary only exists when JavaScript calls Rust — and that is narrower than it
sounds.** Firmware↔desktop is native Rust on both sides with no boundary at all. Firmware↔web
crosses it only for the protocol codec: small, stable, and generated. A pure web product never
crosses it because it never opts in.

Where it is crossed, friction is managed deliberately: types are **generated** by `ts-rs`, never
hand-written; the dev loop is watch-mode `wasm-pack` with hot reload; and the boundary is designed
to be crossed **rarely with meaningful payloads, not constantly with small ones**. A chatty
Rust↔JS API is the thing that makes people hate WASM, and it is an design error, not an inherent cost.

**The `no_std` rule that keeps L0 honest:** every L0 crate is `#![no_std]`,
`default-features = []`, with `alloc`/`std` behind flags. Embedded is the strictest target, so
designing for it yields WASM free. CI compiles the spine for `x86_64`, `wasm32-unknown-unknown`,
`thumbv6m-none-eabi` and `thumbv7em-none-eabihf` **every commit** — it fails the day it breaks,
not the day it is needed.

**Escape hatch (MISSION principle 3):** Python is permitted as a **build-time pipeline executor**
— scientific computing, an OCCT geometry backend, ML training — managed with `uv`, and **never in
the spine or a runtime path**. Rust is the default, not a religion.

---

## 5. Topology

```
AleksaZCodes/fiducial                     public · MIT
├─ MISSION.md · LICENSE (MIT) · IP-POLICY.md
├─ .claude-plugin/marketplace.json
├─ Cargo.toml                             HOST workspace (excludes firmware/)
│
├─ crates/                                ── L0 ──────────────────────────
│  ├─ fiducial-quantity/  no_std  Quantity<D>, tolerance algebra, assertions
│  ├─ fiducial-model/     no_std  Fact/Decision schemas, pipeline contract
│  ├─ fiducial-protocol/  no_std  framing, fragmentation, addressing, codecs
│  ├─ fiducial-core/      no_std  IDs, time, validation, state machines
│  ├─ fiducial-geometry/  no_std  parametric geometry + constraints
│  ├─ fiducial-mesh/      std     optional csgrs CSG — only for no_std-shareable solids
│  ├─ fiducial-sim/       std     numerical, rayon
│  ├─ fiducial-wasm/              wasm-bindgen + ts-rs generation
│  └─ fiducial-tauri/             serial transport, device discovery
│
├─ firmware/                              SEPARATE cargo workspaces
│  ├─ shared/ · rp2040/ (thumbv6m) · stm32/ (thumbv7em)
│
├─ pipelines/            implementations of the §3.2 contract
│  ├─ eda/  mech/  types/  docs/  render/
│
├─ packages/                              ── L1 / L2 / L3 ────────────────
│  ├─ tokens/ headless/ wasm-bridge/ device/ supabase/
│  ├─ i18n/ email/ maps/ testing/ realtime/
│  ├─ viewer3d-react/ viewer3d-svelte/
│  └─ ui-react/ ui-svelte/
│
├─ workbench/  cli/  migrations/  plugin/  templates/  docs/

AleksaZCodes/<product>                    private
├─ MISSION.md  fiducial.lock  fiducial.toml  .claude/settings.json
└─ product/    facts, decisions, pipelines — the declarations
```

**Firmware workspaces are separate on purpose.** One Cargo workspace shares a single
`Cargo.lock` and one `.cargo/config.toml` target; a root `cargo test` would try to build firmware
for the host and fail. Root `Cargo.toml` sets `exclude = ["firmware/*"]`; each firmware workspace
has its own lockfile, toolchain, triple and `runner = "probe-rs run --chip …"`.

---

## 6. Ring of Pursuit — first commercial consumer, and the behavior guarantee

**Migration happens when you decide, and not before. Nothing in Fiducial depends on it** — the
platform is complete and useful without it, and Phase 8 can be deferred indefinitely or dropped.

What is *not* optional, because it has standalone value: **the characterization harness (Phase 6)**.
It is a safety net for any future ROP change, migration or not — the first thing that would have
caught the `buildRoundSnapshot` bug that produced Rounds with zero Sides and zero Markers while
every test stayed green. Build it because ROP deserves it; decide about migration separately.

The cost of deferring, stated plainly: L1/L2 packages then have no production consumer, so
extraction quality stays unverified until some product adopts them.

If and when migration does happen: **code and internals free to change; observable behavior is not.**
It runs real events, so a regression is not a bug report — it is a ruined event.

**What can and cannot be promised.** A behavior-preserving refactor of this size cannot be
guaranteed correct by inspection. What *can* be engineered is that every deviation is detected
before it reaches an event.

1. **Characterize before changing anything.** Golden captures against today's ROP: every server
   action over a corpus of real inputs, a full round-runtime wire transcript for a replayed
   round, rendered HTML per public route, the QR PDF, R2 artifact bytes, every SQL query issued.
   Tests of what it *does*, quirks included — not what it should do.
2. **Freeze external contracts.** Postgres schema, WebSocket shapes, R2 formats (append-only,
   `schema_version`), URLs, cookie names and domains, email templates.
3. **One seam per PR**, green on unit + integration (real Postgres) + E2E (real local Supabase,
   real Durable Object) + characterization, before the next begins.
4. **Differential-test anything moving to Rust.** Both implementations over the same generated
   corpus must agree. TypeScript deleted only after.
5. **Data is never at risk.** No destructive migration, no schema change, no R2 format change.

**Bugs found get fixed, under discipline:** recorded in `docs/behavior-changes.md` (old, new,
why); the characterization test updated in the same commit with the diff visible; fixes shipped
as their own commits, never folded into a refactor.

**The invariant: every behavior difference is either a failing characterization test or a
deliberate ledger entry.** Silent drift has nowhere to hide — stronger than a promise that
nothing changed.

**Rollback.** Every PR independently revertable. ROP pins Fiducial packages to **exact versions**,
never `^`. No migration work lands in the week before an event.

---

## 7. L0 — the Rust spine

**`fiducial-protocol` — the answer to the walkie-talkie bug.** Reusable `no_std`,
transport-agnostic primitives: framing, fragmentation/reassembly, sequencing, addressing,
checksums, backpressure. A product defines its messages on top with `serde` + `postcard`.

| Where | How it consumes the protocol |
| --- | --- |
| Firmware (STM32/RP2040) | Native `no_std` dependency |
| Tauri desktop | Native, over USB serial |
| Browser | WASM, over Web Serial / WebUSB |
| Tauri mobile | **Native Rust**, no WASM |
| Cloudflare Worker | WASM, for devices reporting over Wi-Fi |

**A protocol change becomes a compile error in every endpoint that no longer matches.**

Radio layers are not reinvented: `lora-rs` (`lora-phy`, `lora-modulation`, `lorawan-encoding`,
`lorawan-device`) is `no_std` and Embassy-compatible on STM32 and RP2040; `embassy-net`
(smoltcp) covers IP. `fiducial-protocol` sits above these.

**`fiducial-wasm` — generated types, never hand-written.** `ts-rs` generates TypeScript from the
Rust types. ROP's most instructive failure was a wire protocol declared three times by hand:
typecheck and the integration test stayed green while every client was broken. Hand-writing a
type that crosses this boundary is guard-blocked.

**`fiducial-sim` — one real constraint.** `rayon` does not parallelize in WASM by default; it
needs `wasm-bindgen-rayon` plus COOP/COEP headers on the serving origin. Single-threaded WASM is
the default; parallel is opt-in per app with the header requirement documented.

---

### 7.1 Target matrix — Tauri everywhere except the browser

| Target | Shell | How it reaches L0 |
| --- | --- | --- |
| Browser / PWA | none | **WASM** — the one place the boundary is unavoidable |
| Desktop (macOS · Windows · Linux) | **Tauri** | Native Rust |
| Mobile (iOS · Android) | **Tauri** | **Native Rust** — not a webview calling WASM |
| Edge (Worker / DO) | none | WASM |
| Firmware | none | Native, `no_std` |

**Capacitor is dropped from the platform.** With Tauri on mobile the phone gets a real Rust
process, so `fiducial-protocol` runs natively there — the same crate the firmware and the desktop
use, compiled for ARM. The device path becomes identical on desktop and phone, and WASM survives
only in the browser, which is where it cannot be avoided.

Tauri also wraps a pure-web frontend with a trivial Rust backend, so **a TypeScript-only product
ships a native app without adopting Rust** (§4.1 still holds).

**Costs, recorded rather than glossed:** Capacitor's plugin ecosystem is much larger — camera,
push, filesystem, health — so more mobile plugins will be written in Rust here. And **iOS is the
least-proven path**: `tauri-plugin-blec` reports testing on Windows, Linux, Android and macOS,
with iOS signing friction. Capacitor therefore remains available as an **optional capability**
(§3.3), which is what makes this reversible for one product without reversing it for all.

### 7.2 The embedded stack, and why there is no RTOS

**Embassy is an async executor, not an RTOS, and it replaces one here.** `async`/`await` compiles
each task into a state machine, so tasks carry no per-task stack — and per-task stacks are the
dominant RAM cost of a traditional RTOS. On a 64 KB nRF52 part that difference decides what fits.

**Preemption is available and it is real.** Tasks on one executor are cooperative — a task that
never `.await`s blocks its peers — but multiple executors can run at different NVIC priorities
(`InterruptExecutor`), preempting each other through the same hardware mechanism RTIC uses.

| Situation | Choice |
| --- | --- |
| Default — async I/O, radios, sensors, UI | **Embassy** |
| Compile-time deadlock freedom, formally verified timing | **RTIC** — its SRP locks provably cannot deadlock; Embassy's async mutex can |
| Vendor C middleware assuming an RTOS, or a certified kernel (DO-178C, IEC 61508) | Zephyr · ThreadX · FreeRTOS |
| Memory isolation between tasks | Hubris · Tock |

The ecosystem's own rule, adopted verbatim: *if you need SRP locks use RTIC; otherwise use
`embassy-executor`.*

**The executor is not locked in by the HAL — this is the fact that makes the choice deferrable.**
Every Embassy crate *except* the executor works under any async executor. You can run **RTIC as the
executor with `embassy-stm32` as the HAL**, keeping `embassy-time`, `embassy-sync` and `lora-phy`
untouched. And the L0 crates (`fiducial-protocol`, `fiducial-quantity`, `fiducial-core`) are
`no_std` and executor-agnostic — they do not know an executor exists. So the execution model is a
per-firmware-app decision, revisitable, and it costs no rewrite above the HAL.

**Sizing the decision with numbers rather than adjectives.** Measured async task-switch overhead is
~1.7 µs minimal and ~3.5–4 µs accumulated on a 160 MHz STM32; scale roughly 3× for a 48 MHz part.

| Control-loop period | Embassy overhead | Verdict |
| --- | --- | --- |
| 10 ms (100 Hz) | ~0.1% | Trivially fine |
| 1 ms (1 kHz) — typical flight control | ~1% | Fine, with the loop on a high-priority `InterruptExecutor` |
| 100 µs (10 kHz) | ~10% | Marginal — measure, or move to RTIC |
| < 100 µs | dominant | RTIC, or a bare interrupt handler with no executor |

**Worked cases:**

- **Sensor array — Embassy is the better answer, not merely adequate.** N sensors are N async tasks
  each awaiting I²C/SPI/ADC/timer, costing N state machines rather than N stacks. On 64 KB of RAM
  that decides how many sensors fit. `embassy-sync`'s `PubSubChannel` handles fan-out.
- **Aircraft — splits into three, and only one is hard.** Telemetry, logging and comms: Embassy.
  A 400 Hz–1 kHz IMU→attitude→actuator loop: Embassy on a high-priority interrupt executor.
  **Certified avionics (DO-178C): neither Embassy nor RTIC is certified** — that is
  ThreadX/Zephyr/SafeRTOS territory, and the certification decision dominates every other choice in
  the stack. Experimental and hobby aircraft are unaffected by any of it.

**The three questions that actually decide it**, in order: what is the hardest deadline and what
happens if it is missed (soft / firm / hard)? · is certification required? · is any task
long-running and non-yielding (heavy DSP, crypto, sensor fusion), which is what breaks cooperative
scheduling? If all three answer benignly — and for a sensor array or a radio product they do —
Embassy is correct and the question does not need revisiting.

### 7.3 MCU families are capabilities, not platform decisions

**The specific part does not matter; the stack holding across parts does.** An MCU family is a
**capability** (§3.3) contributing five things and nothing else:

```
capability mcu-<family>
  hal            embassy-stm32 | esp-hal | embassy-rp
  target triple  thumbv7em-none-eabihf | riscv32imac-… | thumbv6m-none-eabi
  flash + debug  probe-rs | espflash
  bootloader     embassy-boot config + partition map
  CI matrix      the triples compiled every commit
  SKILL.md       mandatory
```

Everything above the HAL — `fiducial-protocol`, `fiducial-quantity`, `fiducial-core`, the domain
logic — is `no_std` and executor-agnostic and **does not change when the family does**. Adding a
family later (nRF, Ambiq, anything) is additive, with no change to the core. That is the §3
universality test applied to silicon.

**Declared families:**

| Family | Radio | Toolchain | Notes |
| --- | --- | --- | --- |
| **STM32** — incl. **STM32WL** (Wio-E5 / STM32WLE5JC) | **LoRa + (G)FSK on-die** on WL; none on the general line | Upstream stable | `embassy-stm32`; `lora-phy` over the internal SPI. M4 @48 MHz, 256 KB flash, 64 KB RAM. **No Bluetooth** |
| **ESP32** — incl. **ESP32-S3** (Wio-S3) and the RISC-V line | **Wi-Fi + BLE** | **Split — see below** | `esp-hal` (`no_std`) + Embassy |
| **RP2040 / RP2350** | None (Pico W adds CYW43) | Upstream stable | `embassy-rp`; general compute, USB, PIO |

**Pairing note:** STM32WL gives LoRa without BLE; ESP32 gives BLE and Wi-Fi without LoRa. A
walkie-talkie wanting both is either two parts (Wio-E5 + Wio-S3) or an ESP32 with an external
SX126x. That is a product decision (§3), recorded before layout.

### 7.4 Three ESP32 facts that are load-bearing

**1 · The Rust toolchain forks by architecture.** Xtensa parts — ESP32, ESP32-S2, **ESP32-S3
(Wio-S3)** — require a **forked Rust compiler**, installed with `espup`, because Xtensa codegen is
not in upstream LLVM. The RISC-V line — **C3, C6, H2, P4** — builds with **upstream stable Rust**.

*Consequence for CI:* a project using an Xtensa part adds a second toolchain to the matrix. **Prefer
RISC-V ESP32 parts (C6) when the choice is free**; accept `espup` when S3 is specifically wanted.

**2 · `esp-hal`, not `esp-idf-hal`.** `esp-hal` is bare-metal `no_std` and pairs with Embassy —
matching STM32 and RP2040 exactly. `esp-idf-hal` wraps Espressif's C SDK, brings `std`, and runs
**FreeRTOS underneath**. It is more battle-tested and supports more exotic configurations, but it
forks the model: different ecosystem, different concurrency story, a C dependency in the build.
`esp-hal` keeps one mental model across all three families. `esp-idf-hal` remains the documented
escape hatch when a vendor feature exists only there.

**3 · ESP32's radio stack needs a scheduler, and this is the one place the RTOS answer bends.**
`esp-wifi` has been replaced by **`esp-radio`**, which requires a dynamic allocator *and a
preemptive task scheduler in the application* — the simplest supported option being **`esp-rtos`**.
So on ESP32, using Wi-Fi or BLE pulls in a small preemptive scheduler alongside Embassy. It is
driven by the radio stack, not by our design, and it does not apply to STM32 or RP2040. Recorded
here so it is a known property rather than a surprise on the first BLE build.




**The full stack, layer by layer:**

| Layer | Crates | Note |
| --- | --- | --- |
| Toolchain | `thumbv6m` (M0+) · `thumbv7em-hf` (M4F/M7) · `thumbv8m.main-hf` (M33) | Pinned per firmware workspace |
| Runtime & link | `cortex-m-rt` · `memory.x` · **`flip-link`** · `panic-probe` | `flip-link` inverts the layout so a stack overflow faults instead of silently corrupting statics |
| Bootloader | **`embassy-boot`** | A/B, trial boot, rollback, `ed25519` (§12.3) |
| HAL | `embassy-stm32` · `embassy-rp` · `embassy-nrf` | **The contract is `embedded-hal-async`; the HAL is the swappable tool** (§3.4) |
| Execution | `embassy-executor` · `InterruptExecutor` | See above |
| Time & sync | `embassy-time` · `embassy-sync` | `Channel`, `Signal`, `Mutex`, `PubSubChannel` — `no_std`, async |
| LoRa | `lora-rs` — `lora-phy`, `lora-modulation`, `lorawan-*` | Targets `embedded-hal-async` |
| BLE | **`trouble`** (pure Rust, *not yet qualified*) **or `nrf-softdevice`** (fully certified) | A real fork — see below |
| IP / Wi-Fi | `embassy-net` (smoltcp) · `cyw43` | |
| USB | `embassy-usb` | CDC-ACM, HID, DFU |
| Storage | `embedded-storage` · `sequential-storage` | Flash key-value |
| Observability | **`defmt`** + `defmt-rtt` · **`probe-rs`** | Deferred formatting — the format strings stay on the host, not the device |
| Test | `embedded-hal-mock` (host) · `defmt-test` (on-target) | HIL deferred until a board exists |
| Ours | `fiducial-protocol` · `fiducial-quantity` · `fiducial-core` | The same crates the desktop and browser use |

**Two decisions this forces, and they were wrong in the earlier draft:**

1. **No single declared part does both LoRa and BLE.** RP2040 has *no radio*; STM32WL has LoRa but
   no Bluetooth; ESP32 has BLE and Wi-Fi but no LoRa. A product needing both is two parts
   (Wio-E5 + Wio-S3) or an ESP32 with an external SX126x — a product decision recorded before
   layout, not discovered after (§7.3).
2. **BLE qualification is a real fork.** `nrf-softdevice` is certified today; TrouBLE is pure Rust
   and aiming at qualification but not there yet. For a product that must ship BLE-qualified, the
   SoftDevice path is the answer — recorded as a decision (§3), not discovered at certification.

**The partition map is a declared fact, not a linker detail.** `memory.x` and `embassy-boot`'s
partition table must agree exactly, and a mismatch bricks devices in the field. It is declared once
in `product/` and *derived* into both — the §3 thesis applied to the least glamorous file in the
repository, which is exactly where drift hides.

---

## 8. Electronics ↔ mechanical ↔ physical world

**No EDA tool and no CAD kernel is written.** These pipelines wire existing tools together, which
is where the value is and where nobody else is working.

### 8.1 The bridge is a typed contract

```
atopile  →  KiCad  →  pipelines/eda  →  board.interface.json
                                          ├─ origin            ← the shared datum
                                          ├─ outline[], thickness
                                          ├─ mounting_holes[]
                                          ├─ keepouts[]        ← XY + Z from footprint 3D offsets
                                          ├─ ports[]           ← position, face, aperture
                                          ├─ antenna_zones[]
                                          └─ thermal[]
                                                ↓
                            enclosure.rs — parameters, never literals
                                                ↓  pipelines/mech  (build123d · OCCT)
                    enclosure.step · .stl · .glb · 3mf · exact mass properties
                                                ↓  pipelines/slice (Orca/Bambu CLI, headless)
                          .gcode.3mf · print time · material mass · cost
```

**Geometry backend: build123d (Python/OCCT).** B-rep *and* every mesh export from one model —
STEP for machining and injection, STL for printing, glTF for Three.js and Blender, 3MF for the
slicer. Headless, so it runs in CI.

**Fusion 360 remains the interactive CAD, and is deliberately not a pipeline executor.** Its API
runs *inside* Fusion — licensed, GUI-bound, Mac/Windows — so it cannot run headless. Hand-designed
parts come out of Fusion, generated parts come out of build123d, and **STEP is the interchange
between them.** Neither replaces the other.

**Exact mass properties close a loop.** Mass, volume and centre of gravity become derived facts,
so a weight budget becomes an assertion — `assert assembly.mass <= spec.max_weight` — with
tolerances stacked per §3.1. Slicing extends the same idea: print time, material mass and cost
are pipeline outputs, which means BOM cost and print-time claims are derived, not typed in.

Sources: `kicad-cli` jobsets (Gerbers, drill, BOM, pick-and-place, PDF, **STEP**, **GLB**) plus
direct S-expression parsing of `.kicad_pcb` for Edge.Cuts, mounting holes and footprint 3D-model
Z-offsets. [KiCad StepUp](https://www.kicad.org/external-tools/stepup/) covers FreeCAD round-trip
if a board edge ever needs pushing back.

**GLB collapses a subsystem.** `kicad-cli` exports glTF binary directly, and glTF is what both
Three.js and Blender consume. The PCB → landing-page-render path already exists end to end; it
needs wiring, not building.

### 8.2 Four things that make it actually work

**1 · The origin is declared, not assumed.** ECAD origin ≠ MCAD origin is *the* classic failure —
everything comes out offset by a constant nobody can find. It is in the contract explicitly.

**2 · What is user-facing is a declaration, not a derivation.** The board does not know J1 is the
USB port. `product/` declares roles — *"J1 = usb-c, exposed on face −Y"*, *"U3 = LoRa module,
10 mm antenna keep-out"* — and the pipeline **joins the declaration with KiCad's derived
positions**. Declare-once, exactly.

**3 · Tolerances are declared per manufacturing process, not per part.** `tolerances.toml` carries
`fdm-0.4mm`, `sla`, `cnc`, `injection` profiles: clearance, hole compensation, boss sizing for
heat-set inserts, draft. The generator applies the active profile; the assertions of §3.1 stack
them.

**4 · Measure once per process, not once per part.** Print a calibration coupon, measure your
actual machine, write the numbers into `tolerances.toml` — and every enclosure the system ever
generates is correct for *that* machine. This is the honest form of "design it virtually before
it leaves the factory": physical measurement cannot be deleted, but it can stop being repeated.

### 8.3 Pre-manufacture checks

`fid derive --check` catches the expensive mistakes before a print or a fab order:

- enclosure mesh vs component keep-out collision — *"the electrolytic hits the lid"*
- screw length vs boss depth · minimum wall thickness · unsupported overhang angle
- assertion violations with tolerances stacked (§3.1)
- antenna keep-out intruded by shell, battery or copper

---

## 9. Electronics-as-code: atopile now, Zener on board two

**atopile is the authoring layer; KiCad remains the substrate.** Chosen for the features that
matter most here — units, tolerances, assertions, constraint solving, automatic part picking, and
JLCPCB ordering from CI. Its tolerance model is the *same mental model* as §3.1, which is the
real reason it fits rather than merely being liked.

**Zener** ([Diode Computers](https://github.com/diodeinc/pcb)) is deliberately trialled on the
second board: Starlark language, **Rust compiler**, hermetic and deterministic by design, with an
Anthropic partnership and a shipped Claude Code skill. It aligns with the Rust-everywhere thesis
and its agent tooling is first-class.

**Because both compile down to KiCad, this is reversible at the cost of one board's rewrite, not
a platform rewrite.** That property is why KiCad-as-substrate was worth insisting on.

---

## 10. The workbench — a view, never a database

The goal: one place holding conversations, specs, assets, brand, code and design state — "a place
for mind work, not maintenance".

**The repository is the database; the workbench is a view.** If it owned data it would need
syncing, and syncing is the work this system exists to delete. It holds **no private store**: it
reads git, `product/`, artifacts and the graph (§3), and writes back through the CLI as ordinary
commits and PRs.

| Version | Is |
| --- | --- |
| **v0** | `fid dash` — read-only: roadmap, status, briefs, decisions, CI, artifact freshness, `fid graph` rendered |
| **v1** | Assets and brand, 3D viewer over derived GLB, spec sheets, multi-repo |
| **v2** | Authoring — edit facts, trigger derivations, open agent tasks; every action a commit |

Delivered as a Tauri app (Rust backend already has filesystem and git access) reusing L1 tokens
and an L3 binding. **v0 is small** — mostly a renderer over data the repo already produces.

**Claude conversations** are brought in by exporting session transcripts to `docs/sessions/` as
committed artifacts, opt-in per repo. That keeps "repo is the database" true rather than adding
a second store.

---

## 11. The CLI and propagation

`@fiducial/cli` is a **permanent dependency of every product**, documented in every product's
`AGENTS.md` — not a one-shot scaffolder.

| Command | Does |
| --- | --- |
| `fid new <name>` | Scaffold: pick targets, write tree, `git init`, install, print a checklist. **Creates no cloud resources** |
| `fid add app\|module <target>` | Add a Next / SvelteKit / Tauri / mobile / worker / firmware / pcb app, or a bounded-context module |
| `fid derive [--check]` | Run pipelines; `--check` fails CI on stale artifacts or violated assertions |
| `fid graph` | Emit the facts → pipelines → artifacts DAG. For humans, agents and the workbench |
| `fid upgrade` | Propagation — below |
| `fid doctor` | Drift: outdated deps, un-applied migrations, template files behind upstream, stale artifacts |
| `fid dash` | The workbench |

### How a fix reaches every product

1. **Code → semver.** `fid upgrade` bumps packages and crates. A Mapbox fix is a patch release.
2. **API changes → codemods.** Each version that renames or reshapes ships a migration in
   `migrations/<package>/<version>/`, applied in order (`ts-morph` for TS, AST rewrites for Rust).
   Required only when a change affects more than one product; below that, a changelog note.
3. **Scaffolded files → 3-way merge.** The hard one, and the reason this works. `AGENTS.md`, CI
   workflows, `biome.json`, hooks are **owned by the product** and freely edited, yet upstream
   fixes must still reach them. `fiducial.lock` records the version each file came from; upgrade
   diffs upstream between that version and the new one and **3-way merges** against local edits.
   Same model as `rails app:update`.
4. **Learnings → the Claude Code plugin, free.** Six lines in `.claude/settings.json` and a new
   guard rule or playbook entry is live everywhere on next session, with no upgrade at all.

```json
{
  "extraKnownMarketplaces": {
    "fiducial": { "source": { "source": "github", "repo": "AleksaZCodes/fiducial" } }
  },
  "enabledPlugins": { "fiducial@fiducial": true }
}
```

The plugin carries the `PreToolUse` guard with rules read from `fiducial.toml`, so a product
without a database never sees a migration rule. Portable rules: package-manager discipline · no
unpinned CLI fetches · no direct commits to the default branch · no hand-edits to generated files
· no hand-written migrations · no DDL through a database MCP · **no hand-written derived
artifacts or cross-WASM types**.

> **Known defect to fix during extraction.** ROP's guard regexes the raw command string without
> parsing shell quoting — it blocked this very session twice for the package-manager name
> appearing inside an `echo` string. The extracted guard must tokenize shell input, or match only
> in command position. Recorded because a guard that cries wolf gets disabled, and then it
> protects nothing.

---

## 12. Release, deployment and OTA

**This section exists because of version skew.** A live product is a fleet: devices on firmware
1.2, phones on app 3.0, browsers on 3.1, a desktop on 2.9, a Worker on 4. They must interoperate.
It is the same failure as a protocol written twice — one level up, and harder, because you cannot
recall a device from someone's backpack.

### 12.1 The protocol version is a declared fact; compatibility is an assertion

`protocol.version` is declared once in `product/`. **Every artifact embeds the version it was built
against — derived, never typed in.** On connect, both sides exchange versions and negotiate; an
incompatible pair degrades in a *defined* way and never in an undefined one.

```
assert artifact.protocol_version == product.protocol.version      # built from one truth
assert peer.protocol_version     >= product.protocol.min_supported
```

Breaking the protocol needs a migration entry, exactly like an API codemod (§11) — except firmware
in the field may never take it, so `min_supported` is a real product decision with a real support
cost, recorded as a decision (§3).

### 12.2 One release, many artifacts, one version

`fid release` tags a commit and produces **every** platform artifact from it — each embedding the
same protocol version and the same provenance hash (§3.2) — signs the firmware, publishes, and
records the compatibility matrix as a committed artifact.

| Platform | Mechanism | Rollback |
| --- | --- | --- |
| Web (Vercel / Cloudflare) | Atomic deploy | Instant — previous deployment |
| PWA | Service-worker update with an **explicit** `skipWaiting` policy | Cache version |
| Desktop (Tauri) | `tauri-plugin-updater`, signed binary. **Tauri compiles web assets into the binary, so there is no web-only OTA natively** — a full signed binary is the unit | Reinstall previous |
| Mobile (Tauri) | Store release; signed binary via the updater | Store rollback |
| Worker / Durable Object | `wrangler deploy` | `wrangler rollback` |
| Database | Supabase migrations via Git, forward-only | Compensating migration, never a destructive revert |
| **Firmware** | **`embassy-boot`** — A/B active/DFU partitions, power-fail-safe swap, trial boot, `ed25519` signature verification | **Automatic** — no `mark_booted` within N boots reverts to the previous slot |

### 12.3 Embedded OTA — the six that actually bite

1. **Never brick.** A/B partitions, trial boot, automatic rollback. `mark_booted` is called *only
   after* the new image passes its own self-test — not merely on a successful boot.
2. **Signed, always.** `ed25519` verification before flashing; public key baked into the bootloader,
   private key in the portfolio secret store (§14).
3. **Resumable.** A dropped link mid-transfer resumes from its offset. Restarting a 400 KB image
   over BLE because of one dropout is how field updates get abandoned.
4. **Power-safe.** Refuse to flash below a declared battery threshold — a fact, asserted, not a
   constant buried in firmware.
5. **Bandwidth honesty: OTA over LoRa is impractical for a full image**, and pretending otherwise
   strands a fleet. **The update transport is declared per device** — BLE, Wi-Fi or USB. LoRa
   carries commands and telemetry; it does not carry firmware.
6. **Staged rollout.** A canary cohort first; fleet-wide only once health telemetry holds.

### 12.4 Sync, and why idempotency is the whole trick

**Every state-mutating action tolerates being called twice.** Ring of Pursuit already states this
as a first principle, and it does three jobs at once: offline queue replay, interrupted-OTA
resumption, and at-least-once delivery over a lossy radio. All three are the same requirement, and
a system that has it gets "seamless sync" almost for free.

The offline queue (L2) replays against the **original action timestamp, not sync time** — also from
ROP, where it is the difference between a coherent history and a scrambled one.

### 12.5 CI/CD

Per PR: text pipelines, assertions, lint, typecheck, unit and integration tests. Per tag: binaries,
firmware images, signing, every platform artifact, the compatibility matrix.

**A deploy paths-filter must include the harness, not only the subject.** ROP shipped a fix that
silently never deployed because its filter omitted a workspace — and a skipped job reports as a
successful run. Check the *job* outcome, never the run.

---

## 13. Artifacts, provenance and CI cost

| Class | Examples | Where |
| --- | --- | --- |
| **Small text** | Generated TS types, BOM, spec tables, `board.interface.json`, `STATUS.md` | **Committed.** Diffable, reviewable, `--check`able |
| **Binaries** | Gerbers, drill, STEP, GLB, renders, firmware images | **Release artifacts**, attached on tag, plus a local build cache |

Keeps git permanently small while every fab output stays reproducible and traceable to a commit
via the provenance chain (§3.2).

**CI policy, because hardware pipelines are not free.** Text pipelines and assertions run on every
PR — they are fast and they catch the drift that matters. KiCad and Blender containers run **on
tag**, not per PR. The tradeoff is explicit: a broken binary export is caught at release rather
than at commit, which is acceptable because the *interface contract* (`board.interface.json`) is
checked on every PR and it is the interface, not the render, that breaks the enclosure.

---

## 14. IP and licensing

**Not legal advice. Confirm with a patent attorney before Phase 0 publishes anything.**

**Two tiers, and the architecture already draws the line** — `MISSION.md`'s anti-goals and §18
keep business logic out of the platform for modularity reasons, and that same boundary is the IP
boundary.

| Tier | Contents | Status |
| --- | --- | --- |
| **Public** | CLI, guardrails, propagation, tokens, harness, templates, protocol primitives | Published, **MIT** |
| **Private** | Product repos, domain logic, novel protocols/algorithms, hardware designs | Never published |

Three facts that drive this:

- **Publishing does not surrender copyright.** You remain the owner and may dual-license your own
  work later, because you own it.
- **Public disclosure is prior art against you.** The US allows a 12-month grace period; **the
  EPO and most of the world allow none** — publish before filing and novelty is destroyed
  permanently. **Rule: file a provisional before the code goes public.**
- **MIT was chosen partly because Apache-2.0 contains an express patent grant** (§3). If retaining
  patent rights matters, Apache is the *worse* choice, counterintuitively.

Mechanisms, all cheap: an **IP checkpoint in the PR template** ("novel invention? provisional
filed?"); a **CLA** before accepting any outside contribution, without which dual-licensing later
becomes impossible; **trademark handled separately** — code can be MIT while the brand stays
fully owned.

---

## 15. The rule of two

The dominant failure mode is two years on the platform and nothing shipped. The defense is
structural:

- **Structure upfront, content on demand.** Directories, disciplines and boundaries are
  established early because retrofitting is expensive. They stay empty until needed.
- **A capability enters Fiducial when a real product needs it, and is generalized when a second
  one does.** Until then it lives in the product, product-shaped, and that is correct.
- **Every phase ends usable.** Nothing after Phase 8 is required for Fiducial to pay off.
- `MISSION.md` anti-goal 2 is the tiebreaker when a phase starts to grow.

---

## 16. Build order

**Dependency-ordered.** An earlier draft said "resequence by whichever product is next" — that was
wrong, and the correction matters. **A real product is not one domain; it is every domain at
once, in sync.** The walkie-talkie was PCB *and* firmware *and* protocol *and* web *and*
enclosure *and* marketing, simultaneously. So the sequence below is not a menu of domains to pick
from — it is the order in which capabilities can physically be built, and the milestone that
counts is not a phase number:

> **The bar: one complete product, all its domains in sync, derived from one set of declarations.**
> Until that has happened once, Fiducial is a hypothesis. Phases 0–8 make the claim credible;
> phases 9–16 are what a first hardware product actually consumes; that product is the proof.

Phases may run in parallel where their dependencies allow — the ROP track (6, 8) and the hardware
track (14, 15) share nothing but the CLI and the derive runner, and can proceed independently
once Phase 9 lands.

| Phase | Deliverable | Done when |
| --- | --- | --- |
| **0** | Claim `fiducial` on both registries; create the repo; `MISSION.md`, `LICENSE`, `IP-POLICY.md` | Both `publish --dry-run` pass |
| **1** | Monorepo skeleton: JS + host Cargo workspace, Changesets, release CI | A trivial package publishes and installs in a scratch project |
| **2** | **Vertical slice.** `fiducial-core` (`no_std`) + `fiducial-wasm` + 4-target CI matrix | One function runs in a browser, in Tauri, and blinks an LED on an RP2040 — from one crate |
| **3** | `fid` CLI: `new`, `add`, `doctor`; `fiducial.lock`; guard configurable **and shell-aware** | `fid new` yields a building repo; `fid doctor` clean; the §11 false positive cannot recur |
| **3b** | **The capability mechanism** (§3.3): `fid capability new/check/list`, `fid add`, skill aggregation into the plugin | A capability built from the template installs into a product, activates its guard rules, and its skill is live in the next agent session — with electronics and web both expressed *as* capabilities, not as built-ins |
| **4** | Propagation: codemods + 3-way template merge | An upstream `AGENTS.md` change and a renamed API both land in a scaffolded product, conflict surfaced correctly |
| **5** | Claude Code plugin; portable playbook and conventions as skills | A scratch repo with the six-line block has the guard active |
| **6** | **ROP characterization harness** (§6) | Green against today's unmodified ROP; fails loudly on an injected behavior change |
| **7** | L1 tokens + L2 packages | Each publishes and typechecks in isolation |
| **8** | **ROP migration wave 1** (optional, on your schedule) — one seam per PR | Every PR green on all tiers **and** characterization; ledger holds only deliberate entries |
| **9** | `fiducial-quantity` + `fiducial-model` + `fid derive/graph` + types pipeline | A protocol edit regenerates TS types; a stale artifact or violated assertion fails CI |
| **10** | Next.js + SvelteKit templates + thin L3 bindings | Both render the same tokens and the same headless logic |
| **11** | Tauri desktop + `fiducial-tauri` + `fiducial-protocol` | Desktop app talks to a device over USB serial |
| **12** | `firmware/rp2040` + `stm32`, probe-rs + defmt, LoRa via `lora-rs` | `fid add app firmware` yields a flashable project sharing L0 with the desktop app |
| **13** | Web Serial/WebUSB + BLE transports | Same device reachable from browser and phone with the same codec |
| **14** | EDA pipeline: atopile → KiCad → `board.interface.json` + fab outputs | A board change regenerates every output; `--check` catches staleness |
| **15** | `fiducial-geometry` + `fiducial-mesh` + `viewer3d-*` + tolerance profiles | Board outline → generated enclosure → printable STL **and** a GLB on a marketing page; pre-print checks pass |
| **16** | Workbench v0 | One local read-only view over roadmap, status, decisions, CI, graph, freshness |
| **16b** | **`fid release` + version-skew assertions** (§12.1–12.2): one tag, every artifact, one protocol version | A protocol bump fails the build of any artifact still on the old version; the compatibility matrix is a committed artifact |
| **16c** | **Firmware OTA**: `embassy-boot` A/B, signing, resumable transfer, staged rollout (§12.3) | A device updates over BLE, self-tests, marks booted — and a deliberately broken image rolls back automatically |
| **17** | `fiducial-sim`, `realtime`, workbench v1 | Simulation runs native and in WASM; realtime's three contracts covered by tests |
| **18** | **ROP migration wave 2** (optional) — eligible rules to L0 Rust | Each differential-tested before the TypeScript is deleted |

**Phases 2, 4 and 6 are load-bearing.** If the slice does not compose, the layering is wrong. If
propagation does not work, Fiducial decays into copy-paste within a year. **No ROP code is touched
before Phase 6 is green.**

**Phases 0–8 deliver the whole value proposition.** Phase 18 is optional and never runs during
event season.

### Firmware test tiers

ROP's three tiers do not cover hardware. Two are added: **host-run firmware unit tests** with a
mocked HAL, and **on-target tests** via `probe-rs` + `defmt-test`. Hardware-in-the-loop rigs are
deferred until a board physically exists.

---

## 17. Out of scope

- **ROP's product content** — game logic, schema, migrations, seed data, `PRODUCT.md`,
  `ROADMAP.md`, briefs, archive. ROP *consumes* Fiducial; Fiducial never learns what a Flag is.
- **Any change to ROP's external contracts** during migration (§6).
- **`modules/accounts`** — ROP's encodes a two-audience split with separate cookie domains most
  products will not want. The reusable half is `@fiducial/supabase`.
- **ROP's `biome.json` override block** — ~30 lines disabling a11y rules and others across
  individual `apps/marketing/src/components/*` directories. Lint debt from the landing-page port;
  propagating it would make every future product start pre-broken. The shared preset ships strict.
- **Writing a CAD kernel, an EDA tool, a slicer or a renderer.** Integration only.
- **Foreign CAD import** (STEP/IGES parsing), **PlatformIO / C++ firmware**, **Ruby code**,
  **full cloud provisioning**.

---

## 18. Risks

| Risk | Handling |
| --- | --- |
| **Fiducial becomes the project** | Rule of two (§16); phases end usable; nothing after Phase 8 required |
| **A migration regression ruins a real event** | Highest consequence. No ROP code before Phase 6 green; one seam per PR; contracts frozen; exact pins; nothing near an event; every PR revertable (§6) |
| **Behavior change slips silently** | The §6 invariant: a failing test or a ledger entry, never neither |
| **`no_std` discipline erodes** | Four-target CI every commit |
| **Guard cries wolf and gets disabled** | The §11 shell-aware fix is a Phase 3 acceptance criterion, not a nicety |
| **Derive bypassed under deadline** | `--check` in CI plus a guard rule — the same triple that makes ROP's rules hold |
| **Hardware CI cost** | Text on PR, containers on tag (§14); the interface contract is what is checked per-PR |
| **atopile is young** | KiCad substrate makes it reversible; Zener trialled on board two (§9) |
| **Public disclosure kills a patent** | §15: provisional before publish; private tier never published; PR checkpoint |
| **Codemod authoring burden** | Required only above one affected product (§11) |
| **Secrets across many repos** | `age`/SOPS-encrypted env committed per repo, one key in a password manager. No new vendor |

---

## 19. Open, deliberately

- **Whether products share one Supabase project.** A cost question, answered at the second product.
- **Error tracking.** ROP records this as deliberately undecided; Fiducial inherits the gap.
- **Whether `ui-react` ships components or a shadcn-style registry.** shadcn's model is copy-in,
  not install, which may fight a package. Resolve in Phase 10 with a real second consumer.
- **iOS BLE via `tauri-plugin-blec`.** Reported as the least-tested platform for that plugin. Verify
  on real hardware before a product depends on it; Capacitor remains the fallback capability (§7.1).
- **How far L0 goes.** Rust-first is decided; where a domain stops being worth a WASM crossing is
  per-product judgment, made with the first real case and written down then.
- **ML/science depth.** A training run is a pipeline (§3), so the core already absorbs it. Depth
  waits for a real need (§16).
- **Two-convention period.** ROP stays TypeScript-first while new products are Rust-first. The
  rule for which applies where is written down when Phase 9 lands.

---

## Appendix A · Portable rules extracted from Ring of Pursuit

The final extraction pass. Each earned its place by costing real time at least once, and each is
domain-neutral. **These become the content of the Claude Code plugin's skills (§11)** — that is
their consumer.

### A.1 Silent success is the most expensive bug class

Five instances in one repo, all the same shape: *a green step that quietly did nothing.*

- `const { data }` without `error` — a failed Supabase query returns `null`, `?? []` launders it
  into an ordinary empty array. Produced Rounds with **zero Sides and zero Markers, on every
  Round, for as long as the code existed**, with lint, typecheck, unit and E2E all green.
- A `fetch` that 404s **resolves** — `try/catch` catches network errors, not HTTP status. `res.ok`
  is a separate question.
- A CLI that printed its usage text and **exited 0**, so the wrapper reported "pushed 2 secrets"
  while pushing none.
- `upload-artifact` treats "no files matched" as a warning — four red CI runs uploaded no
  evidence and the step reported success.
- A `paths-filter` miss **skips** the job, and a skipped job reports as a successful run.

**Rules:** a silent fallback is a decision to discard evidence · assert on outcomes, not on
rendering · any "capture diagnostics on failure" step must **fail** when it captures nothing ·
verify with the tool's own read path, never your own summary line · check the *job* outcome,
never the run.

### A.2 A rule with no consumer is tax that looks like rigor

Twenty changesets enforced across twenty PRs produced zero changelog, because nobody ran the
folding step. Before adding any rule, gate or artifact: **who reads the output, and when?**

Corollary — **generated state needs two enforcement paths, because the failure modes do not
overlap:** a pre-commit hook (catches the local path) *and* a CI `--check` (catches a merge made
through a web UI, which never runs a local hook).

### A.3 Boundary rules that transfer verbatim

- **No business logic in an app or a UI package.** A component takes props and renders; if it
  reaches for its own data, that is a boundary violation, not a convenience.
- **One entry point per process, branching internally** — never a second parallel handler. ROP's
  Scan dispatcher resolves catch, collect, open, redeem, revive and capture-damage as one function.
- **Parameters are read from config, never hardcoded** — even the defaults.
- **The app never branches on platform; the adapter does.** `apps/play` contains no
  `isNativePlatform()` check anywhere — the plugin's own web fallback is what makes the same build
  run as a web page and inside a native shell unmodified. **This rule scales directly to Tauri and
  web in Fiducial** — and it is why dropping Capacitor cost the app layer nothing.
- **Test tier is a property of the module, declared.** Mandatory colocated unit tests where the
  logic is "don't get it wrong"; Storybook instead of unit tests where there is no logic to test.
- **When code contradicts the document, the document wins — and gets fixed in the same change.**

### A.4 Verification recipes

`git merge-base --is-ancestor <branch> main` answers "is it merged" in one command · `<tool> --help`
beats memory for a pinned version · `pnpm view <pkg> time` settles "is this deprecated" (asserted
here once, confidently, and wrong) · use Glob/Grep over raw `find`, which ignores `.gitignore` and
returned 68 hits for a file that exists 6 times · **a local reproduction of a CI failure is almost
always cheaper than another push** · reproduce the CI *environment*, not just the CI command.

### A.5 Measure before you tune

Raise a timeout only after measuring what the work costs with the timeout removed — *"it passes at
52.8 s"* and *"it hangs forever"* look identical at a 45 s limit, and only one justifies a bigger
budget. One sample on a loaded machine is noise: a confident diagnosis from a single measurement
was wrong, and deleting the suspected cause made the test *slower*.

### A.6 An error boundary belongs around the thing that can fail

Not around the thing that must survive it. ROP's map error boundary was mounted above the overlays
it existed to protect, so a Mapbox throw deleted the lobby, the HUD and the SOS button — leaving a
player in a field with no way to call for help. The component was correct; its **placement**
inverted it.

### A.7 Roadmaps ordered by theme hide missing substrate

Order by dependency. If item N cannot start until M exists, M is above it — and if M is not on the
list, the list is lying.


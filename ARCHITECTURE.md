# Fiducial — Architecture & Design Guide

**Who this is for:** a new agent starting work on this system, or a human beginning
a new product. Read this before writing code. It is the mental model; the design
spec (`docs/specs/`) is the reference.

The tiebreaker for anything not resolved here is `MISSION.md`. The enumerated
technology choices are in `STACK.md`. The build order is `PHASES.md`.

---

## 1. The single idea

> **Declare each fact once. Derive every artifact from it.**

A fact used by two places belongs in one declared location and is generated into
both. Hand-writing a derived artifact is an error. Every rule in the system exists
to enforce this or to remove work that only existed because it was not enforced.

---

## 2. Five primitives

Everything in the system is one of exactly five things, distinguished by their
lifecycle rule. Get this taxonomy right and the rest follows.

| Primitive | Is | Lifecycle rule |
|---|---|---|
| **Fact** | A declared value — typed, united, toleranced, attributed | Authored by hand, exactly once. **Never generated** |
| **Decision** | A declared judgment with rationale and date | Appended, never edited. Superseded, never deleted |
| **Pipeline** | A pure transformation `(facts, artifacts) → artifacts` | Code. Declares its own inputs and outputs |
| **Artifact** | A derived output | Generated. **Never hand-edited** — guard-blocked |
| **Graph** | The DAG connecting the four above | Derived. Queryable by humans, agents, `fid graph` |

**The universality test:** adding a new domain (chemistry, acoustics, billing) must
require only new fact types and new pipelines — zero change to the core. If you
find yourself modifying a core primitive to support a domain, you have misclassified
something.

### What counts as a fact?

A fact has a **value**, a **unit**, and a **tolerance**. "12" is not a fact.
"12 mm ± 0.1 mm" is.

```
// Not a fact:
const BOARD_THICKNESS = 1.6;

// A fact:
board_thickness: Quantity<Length> = 1.6 mm ± 10%
```

Decisions are facts about judgment: what was chosen, why, and when. They are
committed to `docs/specs/` and never edited — append a superseding decision instead.

---

## 3. Two axes, one grid

Every piece of code lives at a position in a 2-D space. Always know where yours is.

### Axis A — reach (how far does this travel?)

Push behavior down. The lower a layer, the more runtimes it reaches and the longer
it lasts.

| Layer | Contents | Reaches |
|---|---|---|
| **L0** Rust `no_std` | Facts, quantities, protocol, geometry, math, state machines | Browser · Tauri · Workers · Node · STM32 · RP2040 · Ruby — **everything** |
| **L1** Tokens | CSS custom properties, Tailwind preset, 3D materials | Any web framework; render pipelines |
| **L2** Headless TS | Transports, Supabase clients, WASM loading, offline queue | Any JS framework |
| **L3** Framework UI | Components | **One framework.** Deliberately thin |
| **L4** Repo system | CLI (`fid`), guard, codemods, derive runner, CI, templates | Everything, language-agnostic |

L4 is **this repo**. It does not execute business logic; it scaffolds and governs
the repos that do.

### Axis B — domain

A domain is nothing but a set of fact schemas plus a set of pipelines. Electronics,
firmware, web, mechanical, billing — structurally identical. Adding a domain means:

1. Define new fact types (what can you declare in this domain?).
2. Define new pipelines (what does declaring those facts produce?).
3. Package as a capability (see §4).

Nothing in the core changes.

---

## 4. The capability system

A **capability** is the unit of extension. It is versioned, installable, and
self-describing.

### What a capability may contribute

| Contributes | Purpose |
|---|---|
| Template files | Scaffolding written into a product on `fid add` |
| Guard rules | What it forbids — checked by `fid guard-check` |
| Fact schemas | New types a product can declare |
| Pipelines | New derivations those facts enable |
| Crates / packages | Code at whichever layer it belongs |
| Migrations | Codemods for its own upgrades |
| `SKILL.md` | **Mandatory.** How an agent uses this capability correctly |

### The minimum viable capability

**One file: `SKILL.md`.** The manifest is derived from the directory. Everything
else is optional, added when the capability actually grows into it.

A capability that ships code without a `SKILL.md` fails `fid capability check`.
An agent using a capability without a `SKILL.md` must reverse-engineer it every
session — which is the exact work the system exists to eliminate.

### When to create a capability vs. put code directly in a product

- If a second product would need the same thing → capability.
- If it is a third-party tool that needs guard rules + agent instructions → capability.
- If it is product-specific logic that will never generalize → product code.

The rule of two: generalize when the second real need appears, not speculatively.

### Installing a capability

```sh
fid add app next            # installs web-next capability
fid add firmware rp2040     # installs firmware-rp2040 capability
fid capability list --all   # see what is available
fid capability check        # verify installed capabilities are well-formed
```

---

## 5. Where does this code belong? (Decision rules)

These are the concrete questions to ask before writing any new code.

### 5.1 Rust or TypeScript?

| Write Rust (L0) when | Write TypeScript when |
|---|---|
| The same logic must run in more than one runtime, especially a microcontroller | It only runs in a browser or a Node/edge server |
| It is a protocol, a unit-bearing calculation, geometry, or a state machine | It is UI, routing, or server glue |
| Getting it wrong is expensive and the type system earns its keep | Iteration speed matters more than reach |

A pure-web product may use zero Rust. Set `spine.enabled = false` in
`fiducial.toml` and you still get everything: CLI, guard, capabilities,
propagation, tokens, CI. Ring of Pursuit is a permanent example of this.

### 5.2 `no_std` or `std`?

Every L0 crate is `#![no_std]` with `alloc` and `std` behind opt-in features.
The strictest target (embedded) is the default. Design for embedded and you get
WASM free.

```toml
# Correct: embedded is the default
[features]
default = []
alloc   = []
std     = ["alloc"]
```

CI compiles the spine for x86_64, wasm32, thumbv6m, and thumbv7em on every commit.
It fails the day it breaks, not the day it is needed.

### 5.3 Where does shared logic live?

| Logic is... | Put it in... |
|---|---|
| Cross-runtime and correct-by-construction | `crates/fiducial-<name>/` (L0, `no_std`) |
| JS-only transport, client, or framework integration | `packages/<name>/` (L2 or L3) |
| Design tokens (colors, spacing, typography) | `packages/tokens/` (L1) |
| A framework component | `packages/<framework>-ui/` (L3) |
| Product-specific and not shared | `apps/<name>/src/` inside the product repo |

### 5.4 When does a value need a unit?

If the value is physical (length, mass, frequency, voltage, time, temperature),
it **always** needs a unit and a tolerance. A bare number is a bug waiting to
happen when someone assumes the wrong unit or passes it to a different domain.

```rust
// Bug:
fn board_thickness() -> f32 { 1.6 }

// Correct:
fn board_thickness() -> Quantity<Length> { mm!(1.6) ± 10% }
```

### 5.5 New file: fact or artifact?

Ask: "Would a pipeline produce this file from something more fundamental?" If yes,
it is an artifact — do not create it by hand, create its source and run the pipeline.

The guard blocks hand-edits to declared artifact paths. If you think something is
both a fact and an artifact, reconsider your pipeline boundaries.

### 5.6 New decision: where does it go?

All decisions go into `docs/specs/` with a date stamp. Never edit a past decision.
Supersede it with a new one. The history is the record.

---

## 6. The guard

The guard enforces "the rules" at agent time, not just at review time. It runs as
a Claude Code `PreToolUse` hook, calling `fid guard-check` before every Bash
invocation.

### How it works

`fid guard-check` reads the hook payload from stdin, **tokenizes** the command
with a shell-aware parser (not a raw string regex), and checks each token in
command position (argv[0]) against the active rule set.

**The key invariant:** a rule fires only when the forbidden word is in command
position. `echo "use npm install"` does not trigger the npm rule. `npm install`
does. This was the false-positive bug in the original ROP guard, and it is fixed.

### Active rules

Configured in `fiducial.toml [guard]`. Default rules:

| Rule | What it prevents |
|---|---|
| `no-direct-main-push` | Direct git push to the default branch |
| `no-hand-edit-generated` | Hand-editing generated (artifact) files |
| `no-unpinned-cli-fetch` | `curl \| sh` or `wget` without a pinned version |

Capabilities add their own rules on install. A product with no database never
sees migration rules.

### The triple enforcement model

A rule holds because it is enforced three times:

1. **Agent time** — the guard blocks it in the current session.
2. **Commit time** — hooks in `.git/hooks/` or CI pre-commit checks.
3. **CI time** — the pipeline fails if the artifact is stale or the rule is violated.

One of these alone can be bypassed under deadline pressure. All three together
cannot.

---

## 7. Propagation

When a platform change needs to reach every product:

| Change type | Mechanism |
|---|---|
| Package/crate update | `fid upgrade` — semver bump, locked |
| API rename or reshape | Codemod in `migrations/<package>/<version>/` — applies automatically |
| Scaffolded file update | 3-way merge — `fiducial.lock` records the version each file came from; `fid upgrade` diffs upstream and merges against local edits, surfacing conflicts |
| Guard rule or skill update | Claude Code plugin — active on next session, no upgrade step needed |

The `fiducial.lock` model is identical to `rails app:update`. The product owns
its scaffolded files and may edit them freely; upstream improvements still reach
them via merge.

---

## 8. `fid` in a workflow

| Situation | Command |
|---|---|
| Starting a new product | `fid new <name>` |
| Adding a web app | `fid add app next` (or `svelte`, `tauri`, `worker`) |
| Adding firmware | `fid add firmware rp2040` (or `rp2350`, `stm32`, `nrf52`) |
| Checking everything is in order | `fid doctor` |
| Running derivation pipelines | `fid derive` |
| CI freshness check | `fid derive --check` |
| Pulling platform updates | `fid upgrade` |
| Seeing the dependency graph | `fid graph` |
| Checking installed capabilities | `fid capability list` |
| Adding a new capability to the platform | `fid capability new <name>` |

Run `fid --help` or `fid <command> --help` for the full reference including
examples and argument descriptions.

---

## 9. Starting a new product

```sh
fid new my-product          # scaffold the product
cd my-product

# Edit fiducial.toml:
#   - Set spine.enabled = true if you need L0 Rust crates
#   - Leave false for a pure-web product (Ring of Pursuit stays false)

# Edit MISSION.md:
#   - One paragraph: what is this product, who does it serve?

fid add app next            # if it has a web frontend
fid add firmware rp2040     # if it has embedded firmware
fid add app tauri           # if it has a desktop app

fid doctor                  # verify everything is in order
git add -A && git commit -m 'feat: initial scaffold'
git push -u origin main
```

### Checklist for the first session

- [ ] `MISSION.md` has the product's actual mission (not the template placeholder)
- [ ] `fiducial.toml` has the right `spine.enabled` value
- [ ] `AGENTS.md` has been read (it contains the guard rules and key constraints)
- [ ] `fid doctor` is clean
- [ ] The branch + PR workflow is in place (never commit directly to main)

---

## 10. What agents must not do

Regardless of what a user asks, these are never correct:

| Never | Because |
|---|---|
| Hand-edit a file recorded in `fiducial.lock` | It will drift from its upstream template and block `fid upgrade` |
| Write a derived artifact by hand | Change its upstream declaration and run `fid derive` |
| Commit directly to the default branch | `fid doctor` flags it; the guard blocks it |
| Repeat a value from one file in another | Find its declaration; if none exists, create one |
| Supersede a decision by editing the old file | Append a new decision with a date stamp |
| Run `curl \| sh` without a pinned version | Pin it, or use a declared package dependency |
| Write cross-WASM types by hand | They are generated by `ts-rs`; the guard blocks hand-writes |
| Skip reading a capability's `SKILL.md` | It contains product-specific constraints the agent cannot infer |

---

## 11. Where the spec lives

This document is the mental model. For depth, go to:

| What you need | Where |
|---|---|
| The principles (why) | `MISSION.md` |
| Every technology decision | `STACK.md` |
| The full design specification | `docs/specs/2026-09-06-fiducial-design.md` |
| Current phase and deliverables | `PHASES.md` |
| Agent context for this session | `AGENTS.md` (and `.claude/skills/*.md` for installed capabilities) |
| A specific capability's usage | `.claude/skills/<capability-id>.md` |

---

## 12. Adding to this document

This document records the architectural decisions that a new agent or human needs
to get oriented. When a new structural decision is made:

1. Add the decision to `STACK.md` (if it is a technology choice).
2. Add an entry to `docs/specs/` (if it is a design decision with rationale).
3. Update this document only if the decision changes how new code is structured or
   how new agents should reason about the system.

Do not add implementation details here. This is not a reference; it is a map.

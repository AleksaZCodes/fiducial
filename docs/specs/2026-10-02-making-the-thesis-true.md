# Making the thesis true

**Date:** 2026-10-02
**Status:** accepted — the work plan until the paper is written
**Owner's instruction:** fill every gap identified (except slicing), make one
change propagate across domains, build a public example product, remove
what is not needed, fix the bugs — so the paper's premise is *true*.

---

## The premise

The paper is titled **"Fiducial: Build any robust multidisciplinary product
with AI agents."** The title is the thesis and the abstract is built on it.
It must be literally true of the platform at the release the paper cites
(`v0.9.x`, version DOI via Zenodo — `2026-10-02-one-release-per-platform-version.md`).

Read word by word, the title commits to five things; each needs evidence
in this repository, not in a private one:

| The title says | So this must be true | Today |
|---|---|---|
| **Build** | the output is something a manufacturer can make: orderable files, real parts | board Gerbers yes; assembly needs an LCSC number on every part — not automatic (G1) |
| **any** | the method is general, not one product's special cases | rules are generic constraint kinds (P1 done); but the only end-to-end product is private (G3) |
| **robust** | drift cannot ship: gates, checks, contradictions named | true for hardware artifacts; *not* across domains (G2) |
| **multidisciplinary** | one fact feeds several disciplines | hardware ↔ firmware ↔ web are not connected for hardware facts (G2) |
| **with AI agents** | agents do the work inside the gates, and the record shows it | true; needs a public, reproducible account (G3) |

The abstract states only what the table shows as true at the cited release.
Anything still "Today: no" is either done before the release or stated as a
limit.

## Gaps, and how each is closed

### G1 · Parts as packages — every part orderable, resolved like a dependency

**Prior art, and what to adopt instead of inventing:**
- **jlcparts** (yaqwsx/jlcparts): an open, regularly rebuilt database of
  the whole JLCPCB/LCSC catalogue (category, package, value, stock, basic or
  extended). It answers "a 10 kΩ 0603 1 % resistor, basic part" with an
  LCSC number.
- **easyeda2kicad** (uPesy): converts any LCSC part's symbol, footprint and
  3D model into KiCad format — the long tail KiCad's own libraries lack.
- **KiCad's official libraries**, pinned by tag (already used).
- **Cargo / npm lockfiles** as the model: a declared *requirement* resolves
  to an exact *version* recorded in a lock with a hash.
- Seen, not adopted as the backbone: atopile's package registry (its own
  language), Octopart/Nexar (paid API, keyed), SnapEDA (closed).

**Decision:** `parts.py` becomes a resolver with a lock.
- A part may declare a **requirement** instead of an LCSC number:
  `pick = { kind = "resistor", value = "10k", package = "0603", tolerance = "1%", tier = "basic" }`.
  `parts.py resolve` picks the cheapest in-stock match from jlcparts and
  writes it to `hardware/parts.lock`, with the LCSC number, MPN, price at
  the declared quantity and the snapshot date. A part with `lcsc` set is
  pinned.
- A part with only `lcsc` and no KiCad library entry is fetched through
  easyeda2kicad into `hardware/lib/`, hashed in the manifest, as today's
  datasheets are.
- `fid derive --check` fails when `parts.lock` does not answer every
  requirement. `fab.py` then lists no unsourced parts: the assembly BOM is
  complete by construction.
- Offline or blocked: the lock is committed, so CI never needs the network.
  Only `resolve` does — like `cargo update`.

**Done (G1).** `parts.py resolve` answers every `pick` into `parts.lock`;
derive takes the LCSC number from the lock and fails on an unanswered or
changed pick; `resolve` also verifies every pinned `lcsc` against LCSC's own
record (a mistyped C-number, or one naming another MPN, is refused);
`parts.py fetch` vendors an LCSC part through easyeda2kicad with
project-relative paths. Findings:
- The published catalogue was mid-migration on the day it was first read
  (CSV: 453 rows; SQLite: one basic part in 53 012). A committed, sticky
  lock is what keeps a product buildable through a day like that; `resolve`
  is the only step that sees it.
- `has = ["1%"]` first matched "±0.1%" (substring). Description words are
  now matched as whole tokens, and the test catalogue carries that decoy.
- A capability directory read at runtime embedded `__pycache__`, as
  `build.rs` once did; both skip it now.

### G2 · One change propagates across domains

The thesis's flagship example is a pin. Today a pin is declared in
`hardware/product.toml` and reaches the board and schematic, but firmware
and web never see it.

**Decision:** the hardware solve emits `generated/interface.json`. It holds:
- every connector, with its position, opening and connector family;
- every net on the MCU's pins (net → pin name → GPIO);
- the board outline.

From it, `fid derive` writes:
- `firmware/.../board.rs`: one `pub const` per net, `SENSOR_SDA = 4`, for
  the `firmware-rp2040`/`stm32` capabilities;
- TypeScript types for the web, through the existing types pipeline.

Renaming a net, or moving `SDA` to another pin, then changes the board, the
schematic, the firmware constant and the web type in one derive. A firmware
file still using the old name stops compiling. `--check` fails until the
new name is used. That is the demonstration.

**The `eda` capability** (atopile → `board.interface.json` → generated
enclosure via `fiducial-mesh`) overlaps `hardware` and is the README's
worked example. Two ways to make an enclosure contradict "declare once".
**Decision:** `hardware` owns the board and the case. `interface.json` is
derived by it, and that is what `eda`'s `board.interface.json` was for. The
`eda` capability is deprecated, kept installable for one release with a
pointer, then removed. The README's worked example moves to the example
product (G3). `fiducial-mesh` stays as a library: it is what writes
STL/GLB.

### G3 · A public example product, built in this repository

The private reference product cannot be shown. The paper's worked example
is a small, cheap product **in this repo**, built by CI.

**`examples/sensor-stick/`** — a USB-C environment sensor stick:
- **Parts:** RP2040, SHT40 (temperature, humidity), one WS2812 LED, a USB-C
  receptacle through the case wall, an LDO, a crystal, flash and
  passives. Every part from the JLCPCB basic/extended catalogue, picked
  through G1. Bill under about €10 at 5 pieces.
- **Case:** a printed two-part shell with the USB-C opening derived from the
  connector, sealed and checked by `cad.py`.
- **Firmware:** an RP2040 firmware capability that reads the SHT40 and
  sends readings over `fiducial-protocol` on USB serial. Pin constants come
  from G2.
- **Web:** a page that reads the stick over Web Serial
  (`@fiducial/transport-web`), with types from the same declaration.
- **CI:**
  - `fid derive --check` on the example;
  - `build.sh` with `SKIP_RENDER=1`;
  - routed and DRC-clean;
  - the firmware compiles;
  - `fab/README.md` reports zero unsourced parts.

It is **not claimed to have been manufactured or tested on hardware.** The
paper says "orderable and checked", not "built".

The README's worked example and a guide walk through it from `fid new` to
fab files: the commands, the files a person writes, what is derived, and
where an agent's changes land. The same story the deleted private case study
told, about something anyone can clone.

### G4 · The renders

Software WebGL takes about 10 s a view; CI skips renders. Not on the paper's
critical path. Fewer shadow passes in `viewer.html` is the cheap fix, done
when convenient.

## Remove or fix

- **`eda` capability:** deprecate (G2).
- **The `flame` layer name** in the `design` capability's logo scripts (with
  `letters`/`device`) is one product's brand structure hard-coded in a
  platform capability. Generalize to declared layer names (`[brand]
  mark_layer`), defaulting to the current names so existing products keep
  working.
- **Floor placement onto the solver** (engines spec P2). It is the last
  bounded search, and it is why a board could not move under its vent.
- **Determinism:** CP-SAT placement is deterministic; Freerouting is not.
  Record the routed board's hash in the build output, so a difference is
  visible. Hand routing (P4) remains the escape.
- **Wires are checked only against chimneys.** Check every core against
  every solid at its height (`cad.py`), as parts are.
- **Per-package GitHub releases** that exist stay as history; new ones are
  off.
- **Unexplained:** the dev `fid` reports a product's `adapters.generated.ts`
  stale where `0.2.1` does not. Reproduce in a fixture; fix or explain.
- **Not doing:** slicing/print profiles (owner's call); self-hosting fid
  with an older fid (complexity without product value); a landing page.

## Order

Cost of delay first (MISSION 5c):

1. **G1** parts resolver and lock. Debt-accruing: every product added before
   it has to be migrated.
2. **G2** `interface.json` → firmware and web, plus `eda` deprecation.
   Multiplying, and the paper's core claim.
3. **G3** the example product, using G1 and G2.
4. **Fixes:** the flame layer, wire checks, the routed-board hash, P2.
5. **Release `v0.9.x`**, Zenodo DOI, then the paper.

## The paper (notes in `docs/paper/README.md`)

- **Abstract:** the five-row table above, each row as a sentence with its
  evidence, all of it from the example product and the platform's own
  history.
- **Method:**
  - declare once, derive, gate;
  - engines behind contracts (CP-SAT, OpenCascade, KiCad/Freerouting);
  - parts as packages;
  - agents working inside the gates.
- **Evaluation:**
  - the example product end to end;
  - the hardware rules expressed as eight generic kinds;
  - contradictions named;
  - connection length against the hand-written search (13 % shorter, then
    10 % with a warm start);
  - CI history: drift caught.
- **Limits:**
  - one public example and one private product;
  - not manufactured yet;
  - routing not deterministic;
  - power values computed, not simulated.

## Resuming after a context reset

- Branch `claude/elegant-ritchie-bxgij1` in `AleksaZCodes/fiducial`, PR #90
  (green). Read this file, then
  `2026-10-01-hardware-is-solved-by-engines-not-rules.md` (placement, ledger,
  retrospective) and `docs/paper/README.md`.
- The private reference product is never named or described in this
  repository: not in docs, comments, fixtures, PR text or commit messages.
- Start with G1.

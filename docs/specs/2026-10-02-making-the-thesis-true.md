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

**Done (G2).** `fid-hardware` writes `interface.json`, `interface.ts` and,
with `[firmware] mcu`, `board.rs`: per net on an MCU I/O pin, a macro taking
that pin from Embassy's peripherals (`board::sensor_sda!(p)` → `p.PIN_4`).
Macros, not numeric constants, because Embassy's pins are typed singletons —
a number cannot select one. The test
`a_moved_pin_reaches_the_firmware_and_a_renamed_net_breaks_code_still_using_it`
is the demonstration:
- swapping two pins fails `--check` until derived, after which a firmware stub
  compiles unedited against the new pins;
- renaming the net changes the schematic and the TypeScript, and the stub
  still using the old name fails `rustc` naming it.

`eda` carries a `deprecated` note (new in `capability.toml`): `fid add`
warns, and `fid capability list --all` marks it. The README's worked example
is now the hardware one. `docs/guides/first-product.md` and its captures
still walk through `eda`; they move to the example product with G3.

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

**Done (G3).** `examples/sensor-stick/` exists and its CI job (`example`)
covers every claim in its README:
- derive is fresh and the board routes (0 open, DRC clean);
- every solid check passes and every part is orderable;
- the firmware compiles and its host tests pass;
- the page's test passes and it type-checks.

Departures from the plan:
- **Sensor:** an AHT20, not an SHT40; no SHT40 number could be verified at
  LCSC that day.
- **Parts from LCSC's library:** the sensor, the USB-C socket and the flash,
  with symbol, footprint and model made for one another. KiCad's 3D library
  has no model for the latter two.
- **Parts:** pinned and verified, not picked, because the catalogue was
  mid-migration.
- **Bill:** about $5.14 of parts per board at five boards, LCSC prices with
  minimum order quantities.

Platform bugs the example found, each fixed and tested:
- seal and fastener fields required when irrelevant;
- a derived board always square;
- a sealed vent placed with no room for its ring;
- a press-fit skirt cutting the board;
- firmware templates on `fiducial-protocol` 0.1, with ignored profiles;
- `fid upgrade` re-conflicting forever after a resolved conflict;
- no capability `.gitignore`;
- LCSC refusing requests without a User-Agent;
- easyeda2kicad's project-relative paths;
- a wrong note on press-fit lids without a gasket.
- KiCad's `extends` symbols refused, now flattened;
- a footprint's size read from a courtyard that misses its own pads, now
  grown to cover every pad plus KiCad's 0.25 mm margin;
- easyeda2kicad's KiCad 5 footprints losing their nets, now converted on
  `fetch` and refused by name otherwise;
- one product's orange as every product's case and viewer colour, now
  `case.colour` with a neutral default;
- one product's logo structure (`flame`/`letters`/`device`) and colours in
  the design and SEO scripts, now `mark`/`letters`/optional `accent` with
  neutral fallbacks. The private reference product renames its SVG layers
  when it next upgrades.
- a through-wall socket held wholly on the board, its mouth read from its
  courtyard's margin and its window sized for a plug's body: the plug's
  socket stood back from the case's face. Now it hangs past the board edge
  as far as its pads allow (KiCad's 0.5 mm rule, with a margin), its mouth
  is its fab outline's face, and it must be flush within 1 mm, in a window
  of its face plus 1 mm. Inside the wall, that window is a notch open to
  the base's top; the lid's skirt is cut to match, a plug on the lid fills
  the notch down to the window, and `cad.py`'s insertion
  check passes through it. A sealed case refuses the notch. The notch first
  began 0.5 mm inside the mouth, leaving a lip of wall over the socket; it
  now runs through the whole wall.
- an upgrade conflict moved the merge base to upstream but left the
  recorded hash, so a file resolved to exactly the platform's version was
  reported as drift. Both move now.

The guide (`docs/guides/sensor-stick.md`) rebuilds the stick from scratch on
the current platform, with the first build's refusals kept as its case study.

Not done:
- the guide's captures still walk through `eda`;
- a "case wraps the board" sizing mode (the stick declares `width_mm`);
- **a board cut to its case.** The board is always a rectangle. At the
  stick's plug end its corners met the case's chamfer and held the socket
  back, so the example's chamfer was made smaller. A board should stay
  rectangular where it can, and be cut (a chamfer, a notch) where the case
  needs it: the outline offset inward by wall + clearance, intersected with
  the rectangle, written to `Edge.Cuts` and checked like the rest;
- the floor search's 1 mm step (`case.grid_mm`) also held the board back;
  the example sets 0.5. Floor placement on the solver (P2) makes it
  continuous.

### G4 · The renders

Software WebGL takes about 10 s a view; CI skips renders. Not on the paper's
critical path. Fewer shadow passes in `viewer.html` is the cheap fix, done
when convenient.

## Remove or fix

- **`eda` capability:** deprecate (G2).
- **One product's logo structure in the design capability.** Done (G3):
  `mark`, `letters` and an optional `accent`, with neutral fallback colours.
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
- **Python only where an engine requires it.** `cad.py` (build123d on
  OpenCascade), `place.py` (OR-Tools CP-SAT) and `route.py` (KiCad's
  `pcbnew` API, Freerouting) use those engines' maintained interfaces,
  which are Python. `parts.py`, `fab.py` and `ratsnest.py` use none of them
  (HTTP, CSV, SQLite, `kicad-cli`, drawing). They move into `fid` itself, in
  Rust, after the paper: `fid parts resolve|sync|check|fetch` and fab output
  in the build. Not before — they work and are tested, and a port is no
  evidence for the thesis.
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

---
"@fiducial/cli": minor
---

`fid add capability hardware`: a physical product declared once, as shipped
in Fiducial 0.9.1 (`cargo install fiducial-cli --version 0.9.1`).

`hardware/product.toml` names an outline and regions in any SVG, the case,
and every part: an LCSC number or a requirement to pick one by, a KiCad
symbol and footprint, and what each pin connects to. The `fid-hardware`
executor derives from it, all gated by `fid derive --check`:

- `layout.json` (every position and size, with `why`), `bom.csv`,
  `assembly.md`, `board.kicad_pcb` with every footprint placed and every pad
  on its net, and `board.kicad_sch`;
- `interface.json`, `interface.ts` and, with `[firmware] mcu`, `board.rs`:
  one net reaches the board, the schematic, the firmware's pin map and the web
  types, so code using a renamed net stops compiling.

**Placed by a solver.** Board parts and the case floor (sockets and board)
are placed by OR-Tools CP-SAT through `placement-model.json` /
`floor-model.json`; every rule is an instance of eight generic kinds, and a
contradiction fails naming the declarations that cannot hold together.
Answers are cached by the model's hash, so `--check` needs no Python.

**Checked on every derive.** Pinned parts are verified against LCSC and
locked in `hardware/parts.lock`, priced at the minimum order bought, and the
total asserted against `cost.ceiling`. `[[check]]` solves resistive set
points (a USB-C CC pull-down, a regulator divider) from the declared
resistors and fails one outside its window.

**Built.** `hardware/build.sh` routes the board with Freerouting (or checks a
hand-routed board with `board.routing = "hand"`, gated against the
declaration), runs KiCad's DRC, builds the case in build123d with its fit
checks (interference, insertion, lid, screws, seal), and writes Gerbers, the
assembly BOM and placement file, and review renders.

Also: snap-fit boards, through-wall sockets, wires routed round every
obstacle, vents sealed to a board part by a printed chimney, a board cut to
its case (`board.cut_mm`), and escape hatches (`hardware/shapes.py`, a
part's own STEP `model`).

**Pipelines track their inputs.** A pipeline may list `inputs`; their hashes
go into `fiducial.lock [inputs]`, and `fid derive --check` fails naming the
upstream file that moved even when every output still matches its own hash.
`fid upgrade --capability <id>` refreshes one capability and delivers files
it gained after the product installed it.

The worked example is `examples/sensor-stick`: a USB-C sensor stick, built
end to end in CI.

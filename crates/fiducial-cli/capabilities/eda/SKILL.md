# fiducial:eda — EDA Pipeline Skill

This product has the `eda` capability installed. The EDA pipeline links hardware
schematic/layout to the rest of the Fiducial platform via a machine-readable
`board/board.interface.json` file.

## Workflow

```
board/main.ato          ← atopile source (edit this)
  ↓  atopile build
board/board.kicad_sch   ← KiCad schematic (generated)
board/board.kicad_pcb   ← KiCad layout (generated)
  ↓  fid derive          (pipeline: eda — executor fid-validate)
board/board.interface.json  ← connector/pin/net/outline declarations (tracked)
  ↓  fid derive          (pipeline: enclosure — executor fid-mesh)
enclosure/case-base.stl ← base tray with gasket groove (generated)
enclosure/case-lid.stl  ← lid with compression tongue (generated)
enclosure/gasket.stl    ← the seal — PRINT IN TPU (generated)
enclosure/case.glb      ← exploded assembly for web viewers (generated)
fiducial.lock           ← hashes of every generated artifact updated
```

Two pipelines are installed. `eda` validates the board declaration; `enclosure`
derives printable geometry from the `outline` block in that declaration. Both
run in-process — neither needs atopile or KiCad present, so CI stays green
without EDA tooling.

## The sealed case

The generated case is a compression seal, the standard way to make a printed
enclosure water-resistant:

- **Base** — a tray whose rim carries a groove running all the way around.
- **Gasket** — a ring that seats in that groove. **Print it in TPU** or another
  flexible filament. A gasket printed in PLA or PETG cannot compress, so the
  case will not seal.
- **Lid** — a plate with a matching tongue that presses into the groove and
  squeezes the gasket by `gasket_compression` (25% by default).

The seal is what sets the wall thickness: a wall must be thick enough to carry
a printable lip, the groove, and a second lip. That is why a sealed case has
noticeably thicker walls than a plain tray.

Print the base and lid rigid, the gasket flexible. All three come off the same
printer.

Nothing clamps the lid down — there are no screw bosses or clips — so the gasket
is only compressed while something external holds the lid closed. The case is
splash- and dust-resistant by construction; it carries no IP rating and has not
been pressure-tested.

### Connector openings

Every connector that declares a `mount` gets a rectangular hole punched through
the base wall it faces. Openings are sized from the connector's `type`, so a
`usb-c` port cuts a USB-C-shaped hole without anyone typing a dimension.

An opening may not reach the gasket groove: a hole through the sealing rim is a
leak, and `fid derive` refuses to generate one. If a connector is too tall for
the case, raise `headroom_mm` — the error message says so and names the
connector.

The lid never takes cutouts. A hole in the lid is a hole inside the gasket line,
and no amount of compression seals that.

### Standoffs

Declare `standoff_height_mm` and the board is lifted onto four posts, one under
each corner, instead of resting on the cavity floor. The posts are part of the
base, not separate solids dropped on top of it.

Lifting the board lifts the rim with it — headroom is measured above the board —
so a 3 mm standoff makes the whole case 3 mm taller rather than eating 3 mm of
component clearance.

## Commands

| Command | What it does |
|---|---|
| `atopile build` | Compile `.ato` source → KiCad files + `board.interface.json` |
| `fid derive` | Run both pipelines: validate the board, regenerate the enclosure, record hashes |
| `fid derive --check` | Verify every generated artifact matches its hash in `fiducial.lock` (CI uses this) |
| `fid derive --pipeline eda` | Validate `board.interface.json` only |
| `fid derive --pipeline enclosure` | Regenerate the case parts only |
| `fid graph` | Show the full pipeline DAG including both pipelines |

## board.interface.json schema

```json
{
  "schema_version": "1.0",
  "board": { "name": "string", "revision": "string?", "description": "string?" },
  "outline": {
    "width_mm": 100.0,
    "height_mm": 60.0,
    "thickness_mm": 1.6,
    "tolerance": "fdm",
    "enclosure": { "headroom_mm": 10.0, "standoff_height_mm": 3.0 }
  },
  "connectors": [{
    "id": "J1",
    "name": "USB-C",
    "type": "usb-c",
    "pins": [{ "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }],
    "mount": { "side": "south", "offset_mm": 20.0 }
  }],
  "net_classes": [{ "name": "power", "nets": ["PWR_5V", "GND"] }]
}
```

Pin `direction` values: `input`, `output`, `bidirectional`, `power_in`, `power_out`.

## outline — the enclosure declaration

`outline` is optional. Omit it and the `enclosure` pipeline fails with a message
saying there is nothing to derive; declare it and the STL/GLB regenerate from it.

| Field | Required | Default | Meaning |
|---|---|---|---|
| `width_mm` | yes | — | Board width. Must be > 0. |
| `height_mm` | yes | — | Board height. Must be > 0. |
| `thickness_mm` | no | `1.6` | PCB thickness (1.6 mm is standard FR4). Must be > 0. |
| `tolerance` | no | `"fdm"` | Manufacturing process: `fdm`, `resin`, or `cnc`. |

`tolerance` is not cosmetic — it sizes the geometry. Wall and floor thickness
come from the process minimum wall thickness, and the board-to-wall clearance is
twice the process XY accuracy, because the board edge and the printed wall can
each drift by one tolerance. The same board yields a visibly tighter enclosure on
`resin` than on `fdm`.

A non-positive dimension or an unrecognised `tolerance` is rejected by
`fid derive` rather than silently producing a degenerate mesh that slicers accept
and then print as nothing.

### outline.enclosure — what the process cannot imply

Optional, and every field within it is optional. These are the values no
manufacturing process can tell you: how tall your tallest component is, and what
gasket stock you are using.

| Field | Default | Meaning |
|---|---|---|
| `headroom_mm` | `5.0` | Space above the board. **Raise this for tall parts** — the most common override. |
| `lid_thickness_mm` | process min wall | Lid plate thickness. |
| `gasket_width_mm` | `2.0` | Gasket cross-section width. |
| `gasket_height_mm` | `2.0` | Gasket cross-section height, uncompressed. |
| `gasket_compression` | `0.25` | Fraction squeezed when closed. Must be strictly between 0 and 1. |
| `standoff_height_mm` | none | Height of the four corner posts the board rests on. Omit and the board sits on the floor. |
| `standoff_size_mm` | `4.0` | Footprint of each post, square. |

`gasket_compression` of `0` never squeezes the gasket and `1` crushes it flat;
both produce a case that does not seal, so both are rejected.

Everything else — clearance, lip width, floor thickness, gasket fit — comes from
the tolerance class and is not overridable here. Change `tolerance` instead.

## connector.mount — where a connector meets the wall

`mount` is optional, per connector. A connector without one gets no hole — the
right answer for a debug header you reach with the lid off, and the safe default,
because an unnecessary opening is a leak.

| Field | Required | Default | Meaning |
|---|---|---|---|
| `side` | yes | — | Board edge the connector faces: `north`, `south`, `east`, `west`. |
| `offset_mm` | yes | — | Centre of the connector along that edge, measured from the board's origin corner. Must be ≥ 0. |
| `width_mm` | no | family | Body width across the edge. Declare only for a part that differs from its family. |
| `height_mm` | no | family | Body height. |
| `z_offset_mm` | no | `0.0` | Opening floor above the board's top surface. Negative for a mid-mount receptacle that hangs below it. |

`side` and `offset_mm` are in **board coordinates**: the board sits in the first
quadrant, and `offset_mm` runs along the named edge from the board's origin
corner on every side. A connector 20 mm along the south edge and one 20 mm along
the north edge sit at the same board *x* — you never work backwards from the
wall, whose thickness the seal decides.

`width_mm` and `height_mm` are the **connector body**, not the opening. One
process tolerance is added on each side, so the same declaration cuts a tighter
hole on `resin` than on `fdm`.

### Connector families

`type` implies the body envelope, in millimetres:

| `type` | Body | Notes |
|---|---|---|
| `usb-c` | 8.94 × 3.26 | Receptacle shell |
| `micro-usb` | 7.5 × 2.9 | Micro-B shell |
| `usb-a` | 13.2 × 5.8 | Type-A port aperture |
| `swd` | 10.16 × 8.5 | 2×3 0.1" shrouded header |
| `qwiic` | 6.25 × 4.25 | JST-SH 1 mm 4-pin |
| `jst-ph` | 7.8 × 6.0 | JST-PH 2 mm 2-pin |
| `microsd` | 12.0 × 1.6 | Push-push socket |
| `rj45` | 15.9 × 13.5 | 8P8C jack |
| `barrel-jack` | 9.0 × 11.0 | 5.5 / 2.1 mm DC |

A `type` outside this table cannot imply a size, so a mount on one must declare
`width_mm` and `height_mm`. `fid derive` rejects it rather than guessing —
a guessed opening is one the connector may not fit through.

Note the seed board mounts its USB-C and Qwiic ports but **not** its SWD header.
At 8.5 mm tall it would need more than 11.7 mm of headroom to clear the seal on
the FDM profile, where the seed declares 10 — so that header is reached with the
lid off. Raise `headroom_mm` past that and the same declaration builds.

### What `fid derive` rejects

Each of these produces geometry a slicer accepts and a product does not, so all
are caught at the declaration:

| Condition | Why |
|---|---|
| Opening reaches the gasket groove | The case would not seal |
| Opening below the cavity floor | There is no wall there to cut |
| Opening runs off its wall | Nothing to punch through |
| Opening smaller than the process minimum feature | The printer cannot resolve it |
| Two openings on one wall overlap | They merge into a single wide slot |
| Four standoffs will not fit the board | The posts would run into each other |
| Unknown `side`, negative `offset_mm`, non-positive size | Not a placeable opening |

## Rules

- **Never hand-edit `board.interface.json`** — it is generated by atopile from `main.ato`.
  Edit the `.ato` source and run `atopile build && fid derive`.
- **Never hand-edit anything in `enclosure/`** — every file there is derived from the
  `outline` block and the connectors' `mount` blocks. To change the case, change the declaration and re-derive.
  To change *how* geometry is derived from a declaration, change `fiducial-mesh` upstream.
- If `fid derive --check` fails in CI, a generated artifact was edited without
  regenerating and re-running `fid derive` to update `fiducial.lock`.
- KiCad export commands (Gerbers, drill, BOM) can be added as additional `shell` executor
  pipelines in `pipelines/eda-fab.toml`.

## Rendering the case on a web page

`enclosure/case.glb` is a standard glTF 2.0 binary — Three.js, Blender, and
`<model-viewer>` all read it. It holds an exploded view of the three parts,
which is what makes the seal legible at a glance.

With the `web-next` capability installed, `apps/web/src/app/board/page.tsx`
renders it via `@fiducial/viewer3d-react`. There is **no copy step**: uncomment
this line in `pipelines/enclosure.toml` and `fid derive` writes the GLB straight
into the app's public directory.

```toml
outputs = [
  "enclosure/case-base.stl",
  # ...
  "apps/web/public/case.glb",   # ← uncomment if you have a web app
]
```

That copy is a tracked artifact like any other, so `fid derive --check` catches
it drifting. Leave the line commented if the product has no web app — nothing
else depends on it.

## Choosing which parts to generate

Each output is addressed by **stem** and **extension**, and the two are
independent: the stem picks the part, the extension picks the format. So
`enclosure/case-base.glb` is the same geometry as `case-base.stl`.

| Stem | Part |
|---|---|
| `case-base` | base tray with the gasket groove — print rigid |
| `case-lid` | lid with the compression tongue, print orientation — print rigid |
| `gasket` | the seal — **print in TPU** |
| `case` | exploded assembly, for rendering |
| `board` | the bare PCB, extruded |
| `tray` | simple open tray, no seal |

An unrecognised stem fails the pipeline and lists the valid names.

## Adding fab outputs

Create `pipelines/eda-fab.toml`:

```toml
name     = "eda-fab"
executor = "shell"
args     = ["kicad-cli", "pcb", "export", "gerbers",
            "board/board.kicad_pcb", "--output", "board/fab/"]
outputs  = ["board/fab/board-F_Cu.gbr", "board/fab/board-Edge_Cuts.gbr"]
```

Then: `fid derive --pipeline eda-fab`

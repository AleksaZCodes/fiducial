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
    "enclosure": { "headroom_mm": 5.0 }
  },
  "connectors": [{
    "id": "J1",
    "name": "USB-C",
    "type": "usb-c",
    "pins": [{ "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }]
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

`gasket_compression` of `0` never squeezes the gasket and `1` crushes it flat;
both produce a case that does not seal, so both are rejected.

Everything else — clearance, lip width, floor thickness, gasket fit — comes from
the tolerance class and is not overridable here. Change `tolerance` instead.

## Rules

- **Never hand-edit `board.interface.json`** — it is generated by atopile from `main.ato`.
  Edit the `.ato` source and run `atopile build && fid derive`.
- **Never hand-edit anything in `enclosure/`** — every file there is derived from the
  `outline` block. To change the case, change the declaration and re-derive.
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

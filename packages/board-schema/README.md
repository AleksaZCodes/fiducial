# @fiducial/board-schema

TypeScript types and a validator for `board.interface.json` — the machine-readable
declaration of a Fiducial board's connectors, pins, nets, and physical outline.

These types mirror the Rust `fiducial_eda::BoardInterface`. The Rust crate is the
source of truth; this package exists so web code can read the same declaration
without a WASM boundary.

## Install

```sh
pnpm add @fiducial/board-schema
```

## Usage

```ts
import { parseBoardInterface } from '@fiducial/board-schema'
import type { BoardInterface, Outline, PinDirection } from '@fiducial/board-schema'

const board: BoardInterface = parseBoardInterface(await res.text())

for (const connector of board.connectors) {
  console.log(connector.id, connector.pins.length)
}

if (board.outline) {
  console.log(`${board.outline.width_mm} × ${board.outline.height_mm} mm`)
}
```

`parseBoardInterface()` throws rather than returning a partially-valid object. It
checks the schema version, that `board.name` is present, that `connectors` and
`net_classes` are arrays, that a declared `outline` has positive dimensions and a
recognised tolerance class, and that every connector `mount` names a real side,
a non-negative offset, and a body size it can resolve.

Validation mirrors `fiducial_eda::validate` in Rust. The two run on the same seed
file in CI, so a check that exists on one side and not the other shows up as a
test failure rather than as a browser accepting a board the pipeline rejects.

## Schema

```json
{
  "schema_version": "1.0",
  "board": { "name": "my-board", "revision": "A" },
  "outline": {
    "width_mm": 100.0,
    "height_mm": 60.0,
    "thickness_mm": 1.6,
    "tolerance": "fdm",
    "enclosure": { "headroom_mm": 10.0, "standoff_height_mm": 3.0 }
  },
  "connectors": [{
    "id": "J1",
    "name": "USB-C Power",
    "type": "usb-c",
    "pins": [{ "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }],
    "mount": { "side": "south", "offset_mm": 20.0 }
  }],
  "net_classes": [{ "name": "power", "nets": ["PWR_5V", "GND"] }]
}
```

`direction` is one of `input`, `output`, `bidirectional`, `power_in`, `power_out`.

### outline

Optional. When present, `fid derive` generates the sealed case parts from it —
`enclosure/case-base.stl`, `case-lid.stl`, `gasket.stl`, and `case.glb`.

| Field | Required | Default | Meaning |
|---|---|---|---|
| `width_mm` | yes | — | Board width; must be > 0 |
| `height_mm` | yes | — | Board height; must be > 0 |
| `thickness_mm` | no | `1.6` | PCB thickness; must be > 0 |
| `tolerance` | no | `"fdm"` | `fdm`, `resin`, or `cnc` |

`tolerance` drives generated geometry: wall thickness comes from the process
minimum wall, and board-to-wall clearance is twice its XY accuracy. The same
board produces a tighter enclosure on `resin` than on `fdm`.

`outline.enclosure` carries what no process can imply — `headroom_mm`,
`lid_thickness_mm`, the gasket cross-section, `standoff_height_mm` /
`standoff_size_mm`, and `fastener_diameter_mm`. Declaring a standoff height lifts the board onto four corner
posts and grows the case by the same amount, because headroom is measured above
the board.

`fastener_diameter_mm` adds the four corner screws that retain the lid; omit it
and nothing clamps the lid, so the gasket only compresses under external force.
The screws pass through the outer lip, outboard of the gasket groove — a hole
inside the gasket line would open the sealed cavity. The lip has to carry that
hole with a printable wall either side, so declaring a fastener thickens the
case wall.

### connector.mount

Optional, per connector. A connector with a `mount` gets an opening punched
through the case wall it faces; one without gets no hole, which is the right
answer for a header reached with the lid off and the safe default, because an
unnecessary opening is a leak.

| Field | Required | Default | Meaning |
|---|---|---|---|
| `side` | yes | — | `north`, `south`, `east`, or `west` |
| `offset_mm` | yes | — | Centre along that edge from the board's origin corner; must be ≥ 0 |
| `width_mm` | no | family | Connector **body** width; must be > 0 |
| `height_mm` | no | family | Connector **body** height; must be > 0 |
| `z_offset_mm` | no | `0.0` | Opening floor above the board's top surface |

`side` and `offset_mm` are in board coordinates, so an offset means the same
thing on every edge and you never work backwards from a wall whose thickness the
seal decides.

Sizes default to the connector family's body envelope, exported as
`CONNECTOR_OPENINGS` and resolved by `mountEnvelope(mount, type)`:

```ts
import { mountEnvelope } from '@fiducial/board-schema'

mountEnvelope({ side: 'south', offset_mm: 20 }, 'usb-c')
// → { width_mm: 8.94, height_mm: 3.26 }
mountEnvelope({ side: 'south', offset_mm: 20 }, 'db25')
// → undefined — an unlisted family must declare its own size
```

The values are body dimensions, not opening dimensions: the process clearance is
added when the hole is cut, so one declaration yields a tighter opening on
`resin` than on `fdm`.

## Exports

| Export | What it is |
|---|---|
| `parseBoardInterface(json)` | Parse + validate; throws rather than returning a half-valid object |
| `mountEnvelope(mount, type)` | Body size for a mount, from the declaration or the family default |
| `CONNECTOR_OPENINGS` | Body envelopes per connector family, in millimetres |
| `BoardInterface` | Root type of `board.interface.json` |
| `Board`, `Connector`, `Pin`, `NetClass` | The electrical declaration |
| `PinDirection` | `input` \| `output` \| `bidirectional` \| `power_in` \| `power_out` |
| `Outline` | Physical outline that drives enclosure generation |
| `EnclosureOptions` | Case overrides the process cannot imply |
| `Mount` | Where a connector meets the case wall |
| `Side` | `north` \| `south` \| `east` \| `west` |
| `ToleranceClass` | `fdm` \| `resin` \| `cnc` |

Types only — no runtime dependency beyond the validator itself.

## Do not hand-edit the file this parses

`board.interface.json` is generated by atopile from `board/main.ato` and tracked
in `fiducial.lock`. Edit the `.ato` source and re-run `atopile build && fid derive`.

## License

MIT

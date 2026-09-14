# fiducial-eda

**The board interface: one declaration the firmware, the case and the docs all read.**

[![crates.io](https://img.shields.io/crates/v/fiducial-eda.svg)](https://crates.io/crates/fiducial-eda)
[![docs.rs](https://docs.rs/fiducial-eda/badge.svg)](https://docs.rs/fiducial-eda)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

The `no_std` schema for `board/board.interface.json` — the file that states what
a board *is*: its outline, its connectors, their pins and net classes, and where
each connector physically sits.

This file is the declaration that ends the most expensive duplication in
hardware work. A pin assignment normally appears in the schematic, the firmware,
the test rig and the docs; a board dimension appears in the PCB tool, the
enclosure model and the marketing render. Here it appears once, and
[`fiducial-mesh`](../fiducial-mesh) derives the case from it,
[`@fiducial/board-schema`](../../packages/board-schema) reads it in TypeScript,
and `fid derive --check` fails CI when any output stops matching.

## Install

```sh
cargo add fiducial-eda --features std
```

## The schema

```json
{
  "schema_version": "1.0",
  "board": { "name": "my-board", "revision": "A" },
  "outline": {
    "width_mm": 100.0, "height_mm": 60.0, "tolerance": "fdm",
    "enclosure": { "headroom_mm": 10.0, "standoff_height_mm": 3.0 }
  },
  "connectors": [
    { "id": "J1", "name": "USB-C", "type": "usb-c",
      "mount": { "side": "south", "offset_mm": 20.0 },
      "pins": [{ "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }] }
  ],
  "net_classes": [{ "name": "power", "nets": ["PWR_5V", "GND"] }]
}
```

## Validation

`validate()` (needs `std`) is where a declaration is rejected *before* anything
is generated from it. It refuses non-positive dimensions, unknown tolerance
names, unknown connector families and sides, openings that would breach the
gasket seal, openings below the cavity floor, features under the process minimum,
and overlapping openings.

`ValidationError` names the connector and the field to change, because an error
that does not say what to edit is only half an error.

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Types only, pure `no_std`. |
| `alloc` | | Owned strings and vectors. |
| `std` | | Implies `alloc`, enables `validate()`. |

## License

MIT

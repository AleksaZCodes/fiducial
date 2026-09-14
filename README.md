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

## What that looks like in practice

```sh
cargo install fiducial-cli

fid new my-product && cd my-product
fid add eda                # board pipeline + enclosure generation
fid derive                 # run every pipeline, hash every output
fid dash                   # roadmap, decisions, CI, pipelines, freshness
```

Declare a board once in `board/board.interface.json`:

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

Change `width_mm` and every one of those moves. `fid derive --check` fails CI if
one of them didn't.

## The shape of the system

```
        firmware · desktop · browser · edge · CLI     ← many runtimes
                          ╲   │   ╱
                    fiducial-protocol                  ← one waist
                          ╱   │   ╲
        USB serial · Web Serial · WebUSB · BLE · LoRa  ← many transports
```

Logic lives as far down as it can. The `no_std` Rust spine compiles for the host,
`wasm32`, and two embedded targets **on every commit** — so a behaviour written
once runs in a browser, on a desktop, at the edge, and on a microcontroller, and
outlives every framework above it.

## Documentation

| Read | For |
|---|---|
| [`MISSION.md`](./MISSION.md) | Why this exists. The tiebreaker for ambiguous decisions. |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | How the layers fit together. |
| [`STACK.md`](./STACK.md) | Every technology choice, enumerated. |
| [`PHASES.md`](./PHASES.md) | Build order and current state. |
| [`docs/protocol/`](./docs/protocol/) | The wire specification + conformance vectors. |
| [`docs/specs/`](./docs/specs/) | Design decisions, append-only. |

Each crate and package carries its own README — start with
[`fiducial-cli`](./crates/fiducial-cli) if you want to build something, or
[`fiducial-core`](./crates/fiducial-core) if you want to see the spine.

## Repository

| Path | Holds |
|---|---|
| `crates/` | 13 Rust members — the `no_std` spine, the pipelines, the `fid` CLI |
| `firmware/` | A separate Cargo workspace (Embassy; RP2040 + STM32) |
| `packages/` | 11 JS/TS packages — tokens, transports, component registries |
| `docs/` | Specs, the protocol, the compatibility matrix |

## License

MIT — see [`LICENSE`](./LICENSE).

Product repos, domain logic, novel protocols and hardware designs are private;
see [`IP-POLICY.md`](./IP-POLICY.md).

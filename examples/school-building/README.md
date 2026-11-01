# school-building

A school smart-building maquette with these parts:

- a roof controller for irrigation and tank refill;
- one controller per classroom for blinds and the recuperator;
- a Raspberry Pi 4 running the MQTT broker, the server and the app.

It is Fiducial's first distributed product, and the acceptance test for
`docs/specs/2026-11-01-topology-is-a-declaration.md`.

Built on [Fiducial](https://github.com/AleksaZCodes/fiducial).

## Three rules hold the system together

1. A controller decides and acts on its own when Wi-Fi or the server is down.
   It keeps thresholds and the timetable in flash.
2. Anyone can view. Commands, the timetable and thresholds change only for a
   logged-in user, and the server checks every request.
3. ALL STOP and the MANUAL-0-AUTO selectors are hardware and cut contactor
   coil power regardless of any software. AI only describes; it never
   commands.

## What is declared, and what is derived from it

| Declared once | Derived |
|---|---|
| `topology.toml`: nodes, buses, points, rules, parameters, the timetable | per firmware: `firmware/<role>/src/generated/points.rs` and `params.rs`; `crates/skola-messages/src/generated/topics.rs`; `apps/web/src/generated/points.ts` and `cards.ts`; `migrations/0001_topology.sql` |
| `crates/skola-messages`: every message, as Rust types | postcard on device links, JSON at the web; `apps/web/src/generated/wire/*.ts` |
| `messages/*.json`: every string the app shows, including each reason code | `apps/web/src/generated/messages.ts` |
| `hardware/*/product.toml`: modules and wires per node and variant | `firmware/<role>/src/generated/board.rs`; `hardware/*/generated/wiring.csv` and `bom.csv`; `hardware/roof/variants.md` |

`fid derive --check` fails when any of these no longer follows from its
declaration. The message crate's tests fail when a reason code has no message
in a catalog.

## State

| Phase | |
|---|---|
| Messages and topology | done |
| Hardware from modules, with roof variants А and Б | done: `hardware/roof/variants.md` compares them |
| Control rules (`crates/skola-rules`, `no_std`, host-tested, built for the C6) | done |
| ESP32-C6 firmware runtime, self-hosted server, PWA, derived diagrams | to do |

The roadmap is the platform's `ROADMAP.md`, *Distributed, self-hosted
products*.

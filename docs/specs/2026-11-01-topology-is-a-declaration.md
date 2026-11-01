# A system of several computers is one declaration, `topology.toml`

**Date:** 2026-11-01
**Status:** accepted
**Supersedes:** nothing

---

## Context

One product has been one board running one firmware. `fid add firmware <target>`
writes one member per MCU family, and `hardware/product.toml` describes one
board. A distributed product (a building with a controller per room, a
Raspberry Pi server and a web app) has facts
that belong to none of those files:

- which computers exist and what runs on each;
- what connects them, and over which protocol;
- which measured or commanded quantities each owns, with unit, range and
  binding;
- which values a person edits at runtime: thresholds and a timetable.

Without a declaration, those facts were written by hand in seven places:
firmware constants, MQTT topic strings, the broker ACL, `compose.yaml`, the SQL
schema, the web app's cards and 13 hand-drawn diagrams. Six of the 13 diagrams
are fully determined by these facts.

## Decision

`topology.toml` at the product root declares the part of a system every
multi-computer product has:

| Kind | Is |
|---|---|
| `[[system]]` | an isolation boundary: nodes that may share buses and principals |
| `[[node]]` | one computer, MCU or host, with its firmware member or apps, hardware declaration (and variants), and free-text target, runtime and language |
| `[[bus]]` | a physical medium plus the protocol on it, and the node interfaces it connects |
| `[[parameter]]` | a runtime-editable value: unit, range, default |
| `[[topic]]` | a message channel and its direction; generic defaults when none is declared |
| `[[unit]]` | a unit beyond the platform's list |

Everything a product's stack decides is open, not a list the platform
keeps: a node's `target`, `runtime` and `language` are free text, a bus type
is any lowercase name, topics and units are declared, and modules beyond the
platform's library live in the product's `hardware/modules/`. Firmware
tables are generated in the language the output's extension names (`.rs`,
`.h`, `.json`), so a stack the platform has never heard of reads the same
facts and the same `--check` holds it to them.

The `topology` capability's pipeline, executor `fid-topology`, derives from it:

- per firmware role, parameter and pin tables in `.rs`, `.h` or `.json`;
- the product's topic table;
- per hardware variant of a module-built node, a wiring table and diagram, a
  bill of materials, and a cost and pin-headroom comparison.

`fid derive --check` gates every output and adds these validations:

1. nothing crosses a `system`, and MQTT is never without TLS;
2. a parameter's default is inside its range and its unit is known;
3. nodes of one firmware are one firmware;
4. for module-built hardware, the pin rules of the controller (analog needs an
   ADC pin, no output on a strapping or USB pin, no contact on a boot-driven
   pin), bus contracts against the wires, I²C address clashes, supply against
   rail, and cost against ceiling, all from data.

**Monitoring and control is a separate capability.** Points (a measured or
switched quantity), rules, schedules, web cards and the readings schema fit
products that measure things, switch things and show them in an app. They do
not fit a game controller, a flight controller or an audio device, so they
live in the optional `monitoring` capability (`monitoring.toml`, executor
`fid-monitoring`), which requires `topology` and refers to its nodes and
parameters. A product that is not monitoring and control never installs it.

Nodes sharing a `firmware` value share one member. Per-instance identity
(device ID, topic prefix, broker credentials, Wi-Fi) is provisioned at first
flash and never compiled in, so a second room is one `[[node]]` and its
points.

A parameter is a declared fact (unit, range, default) whose *value* is runtime
state. The server is the source of truth for the value; the declaration is the
source of truth for its type and bounds; each node persists the last value it
received.

## Why not the alternatives

- **Extend `hardware/product.toml`.** It describes one board. A bus between
  two boards, or a host with no board, has nowhere to go, and every board
  would restate the others.
- **Put it in `fiducial.toml`.** That file holds per-capability blocks of a
  few keys each. A topology is a list of dozens of records that seven
  pipelines read, which is what a separate declaration file is for (the
  `protocol.toml` precedent).
- **A platform-wide IoT message crate.** No second product needs the same
  messages yet. They stay product-local until one does.

## Consequences

- Changing a pin, a default or a topic changes the firmware tables, the
  wiring and the bill of materials in one derive.
- Adding a point (with `monitoring`) changes firmware, web cards and the
  readings schema in one derive.
- `fiducial-model` is unchanged: a topology is facts and a pipeline. If a
  primitive had to change, the classification would be wrong.

## What this deliberately does not do

- It does not pick or insert parts. A missing transceiver fails the bus
  contract; nothing is added to fix it.
- It does not model the electrical cabinet (24 V ladder, 230 V main circuit).
  That needs a declaration no capability owns yet, and stays a STACK **Gap**.
- It does not segment frames. Only CAN and CAN-FD buses need that, and they
  wait for a product.

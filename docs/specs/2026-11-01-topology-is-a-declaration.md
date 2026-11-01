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

`topology.toml` at the product root declares these kinds:

| Kind | Is |
|---|---|
| `[[system]]` | an isolation boundary: nodes that may share buses and principals |
| `[[node]]` | one computer, MCU or host, with its target triple, runtime, firmware member or apps, and hardware declaration (and variants) |
| `[[bus]]` | a physical medium plus the protocol on it, and the node interfaces it connects |
| `[[point]]` | one measured or commanded quantity: unit, range, tolerance, binding, and the rules it `decides` or `display = true` |
| `[[parameter]]` | a runtime-editable value: unit, range, default, the rule that reads it |
| `[[schedule]]` | a runtime-editable weekly timetable per scope |
| `[[rule]]` | control logic on one firmware role: the points it reads and the actuators it drives |
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

- per firmware role, point, parameter and pin tables in `.rs`, `.h` or `.json`;
- the product's topic table;
- TypeScript point types and app cards;
- SQL migrations for readings, roll-ups, parameters, schedules, command state,
  warnings and descriptions;
- the broker ACL;
- `compose.yaml` and the Caddyfile;
- diagrams.

`fid derive --check` gates every output and adds these validations:

1. connect endpoints resolve to a declared node and an interface its board has;
2. nothing crosses a `system`;
3. bus contracts hold against the netlist;
4. addresses are unique per bus, and nets, channels and units exist;
5. the pin rules of the MCU family hold, as data;
6. parameters, rules and points close: every parameter names a rule, every
   `decides` feeds a rule that reads it, and every rule input is a point;

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

- Adding a point changes firmware, topics, TS types, app cards and migrations
  in one derive.
- The control-rules diagram is derivable, because rules, inputs and parameters
  are closed under validation 6.
- `fiducial-model` is unchanged: a topology is facts and a pipeline. If a
  primitive had to change, the classification would be wrong.

## What this deliberately does not do

- It does not pick or insert parts. A missing transceiver fails the bus
  contract; nothing is added to fix it.
- It does not model the electrical cabinet (24 V ladder, 230 V main circuit).
  That needs a declaration no capability owns yet, and stays a STACK **Gap**.
- It does not segment frames. Only CAN and CAN-FD buses need that, and they
  wait for a product.

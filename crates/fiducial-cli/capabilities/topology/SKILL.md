# fiducial:topology — Topology Skill

This product has the `topology` capability: a system of several computers,
declared once in `topology.toml`. Read
`docs/specs/2026-11-01-topology-is-a-declaration.md` in the platform for the
reasoning. It is general: it does not assume a firmware stack, and it does not
assume the product monitors and controls anything (that is the optional
`monitoring` capability).

## The declaration

| Kind | Is | Keys |
|---|---|---|
| `[[system]]` | an isolation boundary | `id`, `nodes` |
| `[[node]]` | one computer | `id`, `kind` (`mcu` · `host`), `firmware` (MCU), `apps` (host), `hardware`, `variants`, `interfaces`; `target`, `runtime`, `language` as free text |
| `[[bus]]` | a medium and its protocol | `id`, `type` (any lowercase name: `i2c`, `rs485`, `can-fd`, …), `protocol`, `tls`, `broker`, `connect = ["<node>.<interface>"]` |
| `[[parameter]]` | a setting a person edits at runtime | `id`, `scope` (a `firmware` value), `unit`, `range`, `default`, `doc` |
| `[[topic]]` | a message channel between nodes and a server | `id`, `direction` (`up` · `down` · `will`), `retained`, `doc`. None declared: `state`, `command`, `ack`, `parameters`, `schedule`, `online` |
| `[[unit]]` | a unit the platform does not know | `symbol` |

Nodes that share a `firmware` value run one firmware. Per-instance identity
(device ID, credentials, Wi-Fi) is provisioned at first flash, never compiled
in, so a second node of the same firmware is one more `[[node]]` and no code.

## Any firmware

Fiducial does not need to support a firmware stack for the stack to stay
consistent. Tables are generated in the language the output's extension
names, so Embassy, ESP-IDF, Zephyr, Arduino, a vendor SDK or MicroPython all
read the same declaration:

| Extension | For | Shape |
|---|---|---|
| `.rs` | Rust | an enum per table with `id()`, `from_id()` and one `const fn` per field |
| `.h` | C, C++ | an enum and one `#define` per field: `PARAMETER_THRESHOLD_DEFAULT` |
| `.json` | anything else | `{ "entries": [...], "constants": {...} }` |

The shadow-copy scan in `--check` reads Rust, C, C++ and Python, so a pin, an
address or a default typed into hand-written firmware is caught whatever the
language.

## What `fid derive` writes

List each in `pipelines/topology.toml` under `outputs`; the path is the
firmware's own layout.

| Output | From |
|---|---|
| `firmware/<role>/…/params.{rs,h,json}` | the role's parameters: ids, units, defaults, bounds |
| `…/topics.{rs,h,json}` | the topics, their direction and retention, every node's system, target, runtime and language |
| `firmware/<role>/…/board.{rs,h,json}` | the controller pin each wired net lands on (nodes with module-built hardware) |
| `<hardware dir>/generated/wiring.csv`, `wiring.svg`, `bom.csv` | each hardware variant |
| `hardware/<node>/variants.md` | a node with several variants: cost, spare pins, buses |

A firmware role added to `topology.toml` fails derive until its outputs are
listed.

## Hardware built from modules

A node's `hardware` (and each of its `variants`) is a declaration with
`[board] kind = "modules"`:

```toml
[board]
kind = "modules"
mcu  = "mcu"                     # the module that is the controller

[[module]]
id   = "mcu"
part = "esp32-c6-devkitc-1"      # from the module library

[[wire]]
net    = "RELAY_PUMP"
ends   = ["mcu.GPIO18", "relays.IN1"]
use    = "output"                # analog-in | output | contact
colour = "green"
```

`use` picks the pin rules a wire is checked against. Derive refuses:

- an `analog-in` wire on a pin with no ADC;
- an `output` on a strapping or USB pin;
- a `contact` to GND on a pin the boot ROM drives;
- a pin that carries two nets;
- a physical bus (anything but `ip` and `usb`) that connects a node but has no
  module speaking its protocol wired to the controller by a signal wire;
- two modules at one I²C address on one net;
- a module whose supply is not the rail it is wired to;
- a variant's module cost above `[cost] ceiling_rsd`.

The module library is data: the platform's entries (`modules/*.toml`) and the
product's own in `hardware/modules/<id>.toml`, same shape, pin roles included.
An entry there with a platform id replaces it. A controller the platform has
never heard of is a file, not a release.

## What it does not do

It picks no parts and adds nothing to make a check pass. It does not model the
electrical cabinet (ladder logic, mains wiring).

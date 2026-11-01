# fiducial:topology — Topology Skill

This product has the `topology` capability: a system of several computers,
declared once in `topology.toml`. Read
`docs/specs/2026-11-01-topology-is-a-declaration.md` in the platform for the
reasoning.

## The declaration

| Kind | Is | Keys |
|---|---|---|
| `[[system]]` | an isolation boundary | `id`, `nodes` |
| `[[node]]` | one computer | `id`, `kind` (`mcu` · `host`), `target`, `runtime`, `firmware` (MCU), `apps` (host), `hardware`, `variants`, `interfaces` |
| `[[bus]]` | a medium and its protocol | `id`, `type` (`i2c` · `spi` · `uart` · `rs485` · `onewire` · `ip` · `can` · `can-fd` · `usb`), `protocol`, `tls`, `broker`, `connect = ["<node>.<interface>"]` |
| `[[point]]` | one measured or driven quantity | `id`, `node`, `kind` (`sensor` · `actuator` · `contact`), `quantity`, `unit`, `range`, `tolerance`, a binding (`bus` + `address`, `adc`, `pin` or `net`), `calibration`, `interlock`, `display` |
| `[[rule]]` | control logic on one firmware role | `id`, `scope` (a `firmware` value), `reads` (points), `drives` (actuators), `doc` |
| `[[parameter]]` | a value a person edits at runtime | `id`, `scope`, `unit`, `range`, `default`, `rule`, `limits` (the point it draws a limit line on) |
| `[[schedule]]` | a weekly timetable edited at runtime | `id`, `scope`, `granularity` (`"5 min"`), `default_when_missing` (`occupied` · `empty`) |

Nodes that share a `firmware` value run one firmware and must declare the
same points. Per-instance identity (device ID, topic prefix, credentials,
Wi-Fi) is provisioned at first flash, so a second classroom is one more
`[[node]]` and its points: no code changes.

## What `fid derive` writes

| Output | From |
|---|---|
| `firmware/<role>/src/generated/points.rs` | the role's points: enum, units, ranges, bindings, interlocks |
| `firmware/<role>/src/generated/params.rs` | the role's parameters and schedules: defaults, bounds, `accepts()` |
| `…/topics.rs` | topic suffixes, which are retained, every node and its system |
| `…/points.ts` | points, parameters, nodes and rules for the web |
| `…/cards.ts` | one card per sensor; limit lines only for parameters that limit it |
| `migrations/*.sql` | readings, hourly roll-ups, parameter values, schedules, device state, warnings, descriptions |

Every output is gated by `fid derive --check`. A firmware role added to
`topology.toml` fails derive until its two outputs are listed in
`pipelines/topology.toml`.

## What derive refuses

- a node in no system, or in two; a bus or broker crossing systems;
- an MCU target the firmware workspaces do not build, or a host runtime with
  no deploy implementation;
- MQTT without `tls = true`;
- a sensor without a unit, range or binding; a tolerance in another unit;
- two points at one address on one bus (unless they are two quantities of
  one device);
- a rule reading something that is not a point, or driving something that is
  not an actuator;
- a reading no rule uses and not `display = true` (an orphan);
- a parameter naming a missing rule, or with its default outside its range;
- two nodes sharing firmware with different points.

## Hardware built from modules

A node's `hardware` (and each of its `variants`) is a declaration with
`[board] kind = "modules"`:

```toml
[board]
kind = "modules"
mcu  = "mcu"                     # the module that is the controller

[[module]]
id   = "mcu"
part = "esp32-c6-devkitc-1"      # from the platform's module library

[[wire]]
net    = "RELAY_PUMP"            # the net a topology point binds to
ends   = ["mcu.GPIO18", "relays.IN1"]
colour = "green"

[[point]]                        # a variant's own binding for a point
id      = "soil_1"
bus     = "roof_rs485"
address = 1
```

Points bind to **nets** in `topology.toml` (`net = "RELAY_PUMP"`), so every
variant can carry them on different pins. A sensor bound to a net is analog.

Derive checks every variant against the library, where pin rules are data:

- an analog point needs an ADC pin;
- an output on a strapping or USB pin is refused;
- a contact to GND on a pin the boot ROM drives is refused;
- one pin carries one net;
- a bus a point uses needs a module speaking its protocol wired to the
  controller by a signal wire (removing a transceiver fails, naming the bus);
- an I²C point's address needs a module that answers there;
- a module's supply matches the rail it is wired to;
- the variant's module cost stays under `[cost] ceiling_rsd`.

Per variant it derives `<dir>/generated/wiring.csv`, `wiring.svg` (the
controller, each module and every wire, in its colour) and `bom.csv` (price,
vendor, date checked). For the built hardware of each firmware role it
derives `firmware/<role>/src/generated/board.rs` (one constant per net). For
a node with several variants it derives `hardware/<node>/variants.md`, which
compares controller-and-sensor cost, total module cost, spare pins and buses.

## What it does not do

It picks no parts and adds nothing to make a check pass. The electrical
cabinet (24 V ladder, 230 V main circuit) has no declaration yet.

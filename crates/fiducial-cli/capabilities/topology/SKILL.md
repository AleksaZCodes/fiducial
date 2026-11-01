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

## What it does not do

It picks no parts and adds nothing to make a check pass. Bus contracts
against the netlist and MCU pin rules arrive with module-built hardware.

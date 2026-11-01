# fiducial:monitoring — Monitoring Skill

This product has the `monitoring` capability, for a product that measures
quantities, switches outputs, runs rules over them and shows them in an app.
It needs the `topology` capability: it refers to its nodes and parameters. A
product that is not like this (a game controller, a flight controller, an
audio device) does not install it; `topology` alone keeps its nodes, buses,
pins and settings consistent.

## The declaration

`monitoring.toml` at the product root:

| Kind | Is | Keys |
|---|---|---|
| `[[point]]` | one measured or switched quantity | `id`, `node`, `kind` (`sensor` · `actuator` · `contact`), `quantity`, `unit`, `range`, `tolerance`, a binding (`bus` + `address`, `adc`, `pin` or `net`), `calibration`, `interlock`, `display`, `limits` (parameters drawn as limit lines) |
| `[[rule]]` | control logic on one firmware role | `id`, `scope` (a `firmware` value), `reads` (points), `drives` (actuators), `parameters` (topology parameters), `doc` |
| `[[schedule]]` | a weekly timetable edited at runtime | `id`, `scope`, `granularity` (`"5 min"`), `default_when_missing` (`occupied` · `empty`) |

## What `fid derive` writes

| Output | From |
|---|---|
| `firmware/<role>/…/points.{rs,h,json}` | the role's points: ids, units, ranges, bindings, interlocks |
| `firmware/<role>/…/schedules.{rs,h,json}` | the role's schedules |
| `…/points.ts` | points, parameters, nodes and rules for the web |
| `…/cards.ts` | one card per sensor, with the limit lines it declares |
| `migrations/*.sql` | readings, hourly roll-ups, parameter values, schedule slots (SQLite) |

## What derive refuses

- a sensor without a unit, a range or a binding; a tolerance in another unit;
- two points at one address on one bus (unless two quantities of one device);
- a rule that reads something that is not a point, or drives something that is
  not an actuator, or uses a parameter topology does not declare;
- a reading no rule uses and not `display = true` (an orphan), or one a rule
  uses and also `display = true`;
- a parameter no rule reads; a limit line no rule behind that point uses;
- two nodes sharing firmware with different points.

## What it does not do

It does not run rules, store readings or draw the app; it keeps those
consistent with the declaration.

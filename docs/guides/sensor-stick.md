# How the sensor stick was made

[`examples/sensor-stick/`](../../examples/sensor-stick) is a complete product:
a USB-C stick that reports temperature and humidity to a web page, with a
printed case, a routed board, firmware and a page. This is how it was made, in
the order it happened: what a person typed, what an agent did, and each time a
gate said no. It was made by one person and one agent in an afternoon. It has
not been built on a bench, and nothing here claims it works in your hand.

The point is not the stick. It is that every step either wrote a fact once
or derived from one. Each mistake was refused at the step that made it, with
its name, before it could reach the next discipline.

## 1. The product, empty

```sh
fid new sensor-stick && cd sensor-stick
fid add capability hardware
fid add firmware rp2040
```

`fid new` writes the scaffold. The two `add`s install the hardware pipeline
(`hardware/product.toml` → case, board, schematic, BOM, interface) and an
RP2040 firmware workspace. Nothing is drawn yet; the seed declaration is a
placeholder.

## 2. The parts, real ones

**The person** chose what the stick is: an RP2040, a temperature and humidity
sensor, an addressable LED, USB-C. **The agent** found each part's LCSC number,
and checked every number against LCSC's own record before writing it down.
Two candidates were wrong: a sensor number that LCSC does not have, and an LED
with no stock. Both were replaced before they reached the declaration.

The components come from KiCad's own libraries where KiCad has them (the
RP2040, the flash, the crystal, the USB-C socket, the LED). Two do not:

```sh
python3 hardware/parts.py fetch C2757850 C6186
```

That pulled the AHT20 sensor's and the regulator's symbol and footprint from
LCSC's library (easyeda2kicad) into `hardware/vendor/lcsc.*`. The agent
recorded where each came from in `hardware/vendor/SOURCES.md`. KiCad does have
the regulator, but its symbol `extends` another symbol, and `parts.py` refuses
one: it names the parent to vendor rather than inventing the pins.

Then:

```sh
python3 hardware/parts.py sync      # vendor every symbol, footprint, model
python3 hardware/parts.py resolve   # verify every LCSC number → parts.lock
```

`resolve` reported all 21 numbers verified, each with the MPN and package
LCSC gives it.

## 3. The declaration

**The agent** wrote `hardware/product.toml`: 21 board parts, each with its
symbol, footprint, LCSC number and `pins`, mapping each pin to a net. The
regulator's output is `3V3`, the sensor's data line `SENSOR_SDA` on the
RP2040's `GPIO4`, and so on. It also wrote a vent over the sensor that
`seals_to` it, and `[firmware] mcu = "mcu"`. **The person** reviewed the
choices that are design, not wiring: the sensor at the far end from the
regulator's heat, in its own sealed chimney, and the plug at the other end.

## 4. Derive, and what it refused

```sh
fid derive
```

It did not work the first time. Each refusal named what was wrong:

| `fid derive` said | What it meant | Fixed where |
|---|---|---|
| `[case.seal]` is missing `groove_mm` | a case with no seal still had to describe a gasket | **platform**: seal and fastener fields now default; `kind = "none"` needs nothing else |
| no `outline.width_mm` and no part in a region, so nothing sets the size | the stick has no window or panel to size the case from | product: `width_mm` declared |
| the board is 42 × 42 mm, over `board.max_mm` | a derived board was always square | **platform**: one side is held at its ceiling, the other carries the area |
| the chimney's ring and the vent's lean cannot hold together | the vent sat at the very tip, too close to the wall for the ring to land on the board | **platform**: a sealed vent is placed by its chimney's footprint, not its membrane's |
| the USB socket's mouth is 26 mm behind the outer face | the socket was not held against the board's edge, and the board did not reach the wall | product: `faces = "top"` (the mouth, as KiCad draws it), `board.near` includes the plug's wall, and a window sized for the plug's body |

Three of the five were the platform's, not the product's. They were fixed in
the platform, where every product gets the fix, and the decision record says
so (`docs/specs/2026-10-02-making-the-thesis-true.md`).

When it passed, `hardware/generated/` held the case and board layout (`why`
lists what set each size and position), the board with every footprint
placed by the constraint solver and every pad on its net, the schematic, the
BOM, `interface.ts` and `board.rs`.

## 5. Build, route, check

```sh
hardware/build.sh
```

Freerouting routed the board. The first try left connections open: the
RP2040's 0.4 mm-pitch pins do not escape at 0.2 mm tracks with 0.15 mm
clearance. The product moved to 0.15 / 0.127 mm, which JLCPCB's standard
two-layer process holds, and a slightly larger board. KiCad's DRC then
passed.

`cad.py` built the case and checked it, and refused it: the press-fit lid's
skirt, hanging 5 mm into the base just inside the wall, cut through the
board's edge and its snap hooks. Derive should have known, so the fix went
into the platform. The base now grows until the skirt clears the board and
its hooks, and `why` says by how much. Rebuilt, every check passed: nothing
interferes, the lid closes, and the sensor's chimney holds only the sensor.

## 6. Firmware and page, from the same facts

**The agent** wrote the firmware and the page. The firmware takes every pin
from the derived `board.rs`: `board::sensor_sda!(p)` is `p.PIN_4` today. The
page imports the nets and the socket from the derived `interface.ts`. The
sensor's arithmetic and the reading's wire format sit in `firmware/shared`,
tested on the host. The page's decoder has its own test, against the same
bytes.

Writing the firmware surfaced two more platform bugs:
- the firmware templates pinned `fiducial-protocol = "0.1"`, eight releases
  behind;
- their build profiles sat where cargo ignores them.

Both were fixed in the templates. Taking those fixes into the stick with
`fid upgrade` surfaced two more:
- **The upgrade looped.** It conflicted, correctly, on the stick's own edits
  to the same lines. But once the conflict was resolved, every later
  upgrade wrote the same conflict back, because the merge base never moved.
  Now the base moves to the version merged toward, and a file still holding
  markers is refused.
- **Build output was not ignored.** Nothing kept `hardware/build/` and
  `firmware/target/` out of git. Each capability now ships its own
  `.gitignore`.

## 7. Change one thing

This is the claim, run on the stick:

- **Swap two pins in `product.toml`.** `fid derive --check` fails until
  derived. After `fid derive`, the copper, the schematic and `board.rs` have
  moved, and the firmware compiles unchanged.
- **Move SDA to a pin I²C0 cannot use.** The firmware fails to compile,
  because Embassy's types know which pins I²C0 can use.
- **Rename the net.** Every line of firmware or page code still using the
  old name fails to compile.

The test
`a_moved_pin_reaches_the_firmware_and_a_renamed_net_breaks_code_still_using_it`
in `crates/fiducial-cli/tests/hardware_pipeline.rs` runs the same sequence on
every commit.

## Who did what

| | The person | The agent | The gates |
|---|---|---|---|
| What the product is | ✓ | | |
| Which parts | chose the kind | found, verified, wrote | `resolve` verified each number |
| Where things go | judged the design choices | wrote the declaration | the solver placed; derive refused contradictions by name |
| Routing, solids, fit | | ran the build | Freerouting, KiCad DRC, `cad.py` checks |
| Firmware and page | | wrote them | `rustc` and `tsc` against the derived pin map and types |
| Platform bugs found | | fixed in the platform | the next product's derive |

# The sensor stick, from scratch

[`examples/sensor-stick/`](../../examples/sensor-stick) is a complete product:
a USB-C stick that reports temperature and humidity to a web page, with a
printed case, a routed board, firmware and a page. Part 1 builds it again
from nothing, on the current platform, step by step. Each step either writes
a fact once or derives from one. Part 2 is the case study: what happened
when it was built the first time, and what the platform learnt from it.

It has not been built on a bench. Nothing here claims it works in your hand:
it is designed, checked and orderable.

## Part 1 · Build it

You need `fid` (`cargo install fiducial-cli`), Python 3.11+, KiCad 7 or later
(for `kicad-cli` and its libraries), Java 21 (Freerouting, fetched and
pinned by the build), Rust with the `thumbv6m-none-eabi` target, and Node.

### 1. An empty product

```sh
fid new sensor-stick && cd sensor-stick
fid add capability hardware          # hardware/product.toml → case, board, schematic, BOM, interface
fid add firmware rp2040              # an Embassy firmware workspace
pip install -r hardware/requirements.txt easyeda2kicad
```

### 2. Choose the parts, and make them real

What the stick is, chosen by a person:
- an RP2040 with its crystal and QSPI flash;
- a 3.3 V regulator from USB's 5 V;
- an AHT20 temperature and humidity sensor;
- a WS2812B LED;
- a USB-C socket;
- the passives each of those needs.

Each part names a KiCad symbol and footprint, and an LCSC number. KiCad's own
libraries cover the RP2040, the regulator, the crystal, the LED and the
passives, with 3D models. The rest come from LCSC's library, which has a
symbol, a footprint and a model made for one another:

```sh
python3 hardware/parts.py fetch C2757850 C165948 C179173   # AHT20, USB-C socket, flash
```

It lists the library ids to declare (`lcsc:AHT20_C2757850`, …). KiCad has no
AHT20. For the socket and the flash, KiCad's footprints exist but its 3D
library has no model for them, so they would render as boxes. Write down
where each came from in `hardware/vendor/SOURCES.md`.

### 3. Declare it

Two files, copied from the example or written by an agent from the choices
above:
- `hardware/outline.svg`: the stick seen from above, a chamfered bar;
- `hardware/product.toml`: every part, and what each pin connects to.

Excerpts:

```toml
[outline]
svg      = "hardware/outline.svg"
layer    = "outline"
width_mm = 36.0

[case]
colour = "#1d4ed8"           # translucent blue PETG: the LED shows through

[case.fasteners]
closure = "press-fit"        # no screws

[[part]]
id        = "mcu"
symbol    = "MCU_RaspberryPi:RP2040"
footprint = "Package_DFN_QFN:QFN-56-1EP_7x7mm_P0.4mm_EP3.2x3.2mm"
lcsc      = "C2040"
pins      = { GPIO4 = "SENSOR_SDA", GPIO5 = "SENSOR_SCL", GPIO16 = "LED_DIN", ... }

[[part]]
id       = "vent"
mount    = "vent"
seals_to = "sensor"          # a chimney from the lid down to a ring round the sensor

[firmware]
mcu = "mcu"
```

The sensor sits at the far end from the regulator, in its own sealed chimney,
so it reads room air rather than the board's heat. The plug's window is cut
for a USB-C plug's moulded body.

### 4. Vendor, verify, derive

```sh
python3 hardware/parts.py sync       # every symbol, footprint, 3D model → hardware/lib/
python3 hardware/parts.py resolve    # every LCSC number checked against LCSC's record → parts.lock
fid derive                           # case, placed board, schematic, BOM, interface.ts, board.rs
```

`resolve` reports each number verified, with the MPN and package LCSC gives
it. `fid derive` solves the case around the parts and places the board with
a constraint solver. `layout.json → why` says what set each size and
position. `fid derive --check` fails from then on whenever an artifact no
longer matches the declaration.

### 5. Build, route, check, fab

```sh
hardware/build.sh
```

What it does:
- **Routes** the board (Freerouting) and checks it with KiCad's DRC.
- **Builds** the case and checks it: nothing interferes, the board snaps in,
  the lid closes, and the sensor's chimney holds only the sensor.
- **Writes** `hardware/build/fab/`: Gerbers, the assembly BOM and the
  placement file. Its README says whether every part can be ordered for
  assembly.
- **Renders** the review images.

### 6. Firmware and page, from the same facts

The firmware includes the derived pin map and takes every pin through it.
`board::sensor_sda!(p)` is `p.PIN_4`:

```rust
#[path = "../../../hardware/generated/board.rs"]
mod board;
let i2c = I2c::new_async(p.I2C0, board::sensor_scl!(p), board::sensor_sda!(p), Irqs, config);
```

The sensor's arithmetic and the reading's wire format live in
`firmware/shared`, tested on the host. The page imports the nets and the
socket from the derived `interface.ts`, and its decoder is tested against
the same bytes.

```sh
(cd firmware && cargo build --release && cargo test -p firmware-shared --target x86_64-unknown-linux-gnu)
node --experimental-strip-types --test web/src/reading.test.ts
npx esbuild web/src/main.ts --bundle --format=esm --outfile=web/dist/main.js --tsconfig=web/tsconfig.json
```

### 7. Change one thing

- **Swap two pins in `product.toml`.** `fid derive --check` fails until
  derived. After `fid derive`, the copper, the schematic and `board.rs` have
  moved, and the firmware compiles unchanged.
- **Move SDA to GPIO6.** The firmware fails to compile:
  `PIN_6: SdaPin<I2C0>` is not satisfied, because GPIO6 is not one of I2C0's
  pins. Move it to GPIO8 and it compiles again with no edit.
- **Rename the net.** Every line of firmware or page code that still uses the
  old name fails to compile.

The test
`a_moved_pin_reaches_the_firmware_and_a_renamed_net_breaks_code_still_using_it`
runs the same sequence on every commit. The example's CI job runs every step
above.

## Part 2 · The case study: building it the first time

The stick was first built by one person and one agent in an afternoon:
- **The person** chose what it is and judged the design choices: the
  sensor's isolation, where the plug goes, the colour.
- **The agent** found and verified the parts, wrote the declaration, the
  firmware and the page, and ran the gates.

Each mistake was refused at the step that made it, by name.

### What the gates refused

| Step | The refusal | The cause | Fixed in |
|---|---|---|---|
| parts | two candidate LCSC numbers did not verify | one does not exist; one had no stock | product: other parts |
| parts | the regulator's KiCad symbol `extends` another | `parts.py` refused rather than guess its pins | **platform**: `extends` is flattened |
| parts | the socket, flash and sensor had no 3D model | KiCad's 3D library lacks them | product: fetched from LCSC's library |
| route | 64 connections open after fetching from LCSC's library | easyeda2kicad writes KiCad 5 footprints, which lose their nets in a current board | **platform**: `fetch` converts them with `kicad-cli fp upgrade`; a KiCad 5 footprint is refused by name |
| route | still dozens open, pads too close for DRC | the fetched footprints' courtyards miss their own pads, so neighbours were placed on that copper | **platform**: a footprint's size covers every pad plus KiCad's 0.25 mm margin |
| derive | `[case.seal]` is missing `groove_mm` | a case with no seal still had to describe a gasket | **platform**: seal and fastener fields default |
| derive | the board is 42 × 42 mm, over `board.max_mm` | a derived board was always square | **platform**: one side held at its ceiling, the other carries the area |
| derive | the chimney's ring and the vent's lean cannot hold together | the vent sat too close to the wall for its ring | **platform**: a sealed vent is placed by its chimney's footprint |
| derive | the socket's mouth is 26 mm behind the wall | the socket was not held to the board's edge | product: `faces`, `board.near`, a window for the plug's body |
| route | connections left open | 0.4 mm-pitch pins at 0.2 / 0.15 mm rules | product: 0.15 / 0.127 mm, JLCPCB's standard process |
| checks | the lid's skirt cuts the board and its snap hooks | derive did not know about the skirt | **platform**: the base grows until the skirt clears |
| firmware | `fiducial-protocol = "0.1"`, profiles ignored | stale firmware templates | **platform**: templates on the platform's version |
| upgrade | the same conflict, written back after it was resolved | the merge base never moved | **platform**: the base moves; markers are refused |
| doctor | the declaration and the firmware reported as drift | they were classed as platform-owned | **platform**: product-owned |
| render | the case was another product's orange | a platform default copied from a product | **platform**: `case.colour`, a neutral default |

Twelve of the fifteen were the platform's. Each was fixed in the platform,
where every later product gets the fix, with a test. The decision record is
`docs/specs/2026-10-02-making-the-thesis-true.md`.

### Who did what

| | The person | The agent | The gates |
|---|---|---|---|
| What the product is | ✓ | | |
| Which parts | chose the kind | found, verified, wrote | `resolve` verified each number |
| Where things go | judged the design choices | wrote the declaration | the solver placed; derive refused contradictions by name |
| Routing, solids, fit | | ran the build | Freerouting, KiCad DRC, `cad.py` checks |
| Firmware and page | | wrote them | `rustc` and `tsc` against the derived pin map and types |
| Platform bugs found | | fixed in the platform | the next product's derive |

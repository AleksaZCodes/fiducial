# fiducial:protocol — Protocol Skill

This product has the `protocol` capability. **One declaration, many
derivations** — `MISSION.md` principle 1, applied to the bytes a device and
its apps exchange.

## The declaration

`protocol.toml` at the product root lists every message:

```toml
[[message]]
name   = "reading"          # snake_case
kind   = 1                  # the payload's first byte, unique per message
doc    = "One temperature and humidity reading."
fields = [
  { name = "centi_celsius",    type = "i16", doc = "°C × 100" },
  { name = "centi_percent_rh", type = "u16", doc = "%RH × 100" },
]
```

A payload is the `kind` byte, then each field in order, little-endian.
Types: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`.

A field may declare `scale` (what one count is worth: `0.01` for hundredths)
and `unit` (`"°C"`). The web side then gets `READING_UNITS` and
`readingValues(m)`, the fields in their units, so no page divides by hand. A
name that starts with an SI prefix must agree with its scale:
`centi_percent_rh` is `0.01`, and `scale = 0.1` there fails derive.

Thresholds are declared too, in the field's unit, as named bands:

```toml
[[band]]
name  = "comfortable"
field = "reading.centi_percent_rh"
min   = 35.0          # %RH, inclusive
max   = 60.0
```

The firmware gets `reading::COMFORTABLE_MIN` / `_MAX` in counts (3500, 6000);
the page gets `READING_BANDS.comfortable` in %RH. **Test a band with its
generated function** — `reading::comfortable(counts)` in firmware,
`readingBand('comfortable', value)` on the page, both returning below /
inside / above with the bounds inside — not with the bounds: code that
compares against a band's min or max by hand fails derive and `--check`
(`// fid: allow-band` for a line that only shows them). A band bound the
field's type cannot hold (a band written in counts, not in the unit) fails too. Comparing a scaled field
with a bare number in code (`match r.centi_percent_rh { 35..=60 => … }`,
`r.centiPercentRh > 6000`) fails derive and `--check`: the number is in one
unit and the field in another. Zero is the same in every unit and is allowed;
a deliberate line says `// fid: allow-units`.

The USB IDs a device enumerates with, which its page filters on, are one
number on two sides:

```toml
[usb]
vendor_id  = 0x2e8a
product_id = 0x000a
```

Both sides get `USB_VENDOR_ID` and `USB_PRODUCT_ID`. A literal ID in
`Config::new(…)` or a `usbVendorId:` filter fails derive (`// fid: allow-usb`
for a deliberate one).

## What is derived

| File | For | Gives |
|---|---|---|
| `protocol/messages.rs` | firmware (`no_std`, no dependencies) | `mod reading { KIND, LEN, struct Reading, encode, decode }` |
| `protocol/messages.ts` | web | `READING_KIND`, `READING_LEN`, `interface Reading`, `decodeReading`, `encodeReading` |

Include the Rust side with
`#[path = "../../protocol/messages.rs"] mod messages;` (adjust the path), and
import the TypeScript side directly.

## Rules

- **Never write a payload layout by hand.** Not in the firmware, not in the
  web page. If a message is missing, declare it; `fid derive` writes both
  sides.
- **Never edit `protocol/messages.*`.** They are derived; `fid derive --check`
  fails on an edit.
- **Changing a field changes both sides at once.** Renaming one breaks every
  caller at compile time and type-check time — that is the point.
- Framing (start byte, length, CRC) is `fiducial-protocol`'s job; this
  capability is the payload inside a frame.

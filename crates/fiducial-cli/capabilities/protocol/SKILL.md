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

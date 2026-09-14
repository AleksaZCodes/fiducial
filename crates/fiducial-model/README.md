# fiducial-model

**The types that make "one declaration, many derivations" concrete.**

[![crates.io](https://img.shields.io/crates/v/fiducial-model.svg)](https://crates.io/crates/fiducial-model)
[![docs.rs](https://docs.rs/fiducial-model/badge.svg)](https://docs.rs/fiducial-model)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

Three small `no_std` types that give the platform's thesis a representation in
code:

| Type | Is | Answers |
|---|---|---|
| `Fact<T>` | a named, typed value declared once | *what is true?* |
| `Decision` | a judgment with rationale and a date | *why did we choose this?* |
| `PipelineMeta` | what a pipeline consumes and produces | *what follows from it?* |

`Decision` exists because of [principle 1b](../../MISSION.md): *a judgment that
cannot be derived is still declared.* Undocumented judgment gets re-litigated —
by you in six months, and by every agent that follows. `Decision::superseding`
records that a choice replaced an earlier one rather than editing the earlier one
away, because the history is the record.

All types use only `'static` references, so they cost no heap on an MCU.

## Install

```sh
cargo add fiducial-model
```

## Use

```rust
use fiducial_model::{Fact, Decision};

// A fact declared once, and derived from everywhere else.
const SAMPLE_RATE: Fact<u32> = Fact::new("sample_rate_hz", 48_000);
assert_eq!(SAMPLE_RATE.value, 48_000);

// A judgment that no pipeline could have derived.
const RADIO: Decision = Decision::new(
    "2026-09-10",
    "LoRa over BLE for the field units",
    "Range dominates: BLE cannot cross the 2 km site, and the units are \
     battery-fed so a mesh repeater is not acceptable.",
);
assert_eq!(RADIO.date, "2026-09-10");
```

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. |
| `alloc` | | Owned variants. |
| `std` | | Implies `alloc`. |

## License

MIT

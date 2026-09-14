# fiducial-quantity

**A physical fact is never a bare number.**

[![crates.io](https://img.shields.io/crates/v/fiducial-quantity.svg)](https://crates.io/crates/fiducial-quantity)
[![docs.rs](https://docs.rs/fiducial-quantity/badge.svg)](https://docs.rs/fiducial-quantity)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

`no_std` typed quantities that carry their **unit** and their **tolerance**, with
assertions that compare worst cases rather than nominal values.

This exists because the questions that actually matter in a cross-domain product
— *will it fit, will it last, may I claim it* — are answered by stacking
tolerances across domains, not by comparing nominal numbers. A 3.3 V rail that is
"3.3 V ±5%" and a part with a 3.2 V minimum do not conflict on paper and do
conflict in the field. Nominal comparison cannot see that; this crate can.

The dimension is a **type parameter**, so adding a `Length` to a `Frequency` is a
compile error rather than a silent unit bug.

## Install

```sh
cargo add fiducial-quantity
```

## Use

```rust
use fiducial_quantity::{Quantity, Voltage};

// A rail declared as 3.3 V ±5%.
let rail: Quantity<Voltage> = Quantity::with_pct(3.3, 5.0);

// The tolerance band, not the nominal value.
assert!(rail.min() < 3.3);
assert!(rail.max() > 3.3);

// A part that needs at least 3.2 V, exactly.
let required: Quantity<Voltage> = Quantity::exact(3.2);

// Asserts on the WORST case: rail.min() vs required — this is the whole point.
assert!(rail.assert_ge(&required).is_err());
```

That last assertion failing is the crate working. `3.3 × 0.95 = 3.135 V`, which
is below the 3.2 V the part requires — a margin violation that comparing `3.3 >
3.2` would have passed.

## Dimensions

`Length`, `Frequency`, `Time`, `Mass`, `Power`, `Voltage`, `Current`,
`Temperature`, `Dimensionless`.

## Tolerance forms

| Constructor | Meaning |
|---|---|
| `Quantity::exact(v)` | No tolerance — a declared, exact figure |
| `Quantity::with_abs(v, t)` | `v ± t` in the same unit |
| `Quantity::with_pct(v, p)` | `v ± p%` |

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. |
| `alloc` | | Owned types. |
| `std` | | Implies `alloc`. |

## License

MIT

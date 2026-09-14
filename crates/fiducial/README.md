# fiducial

**Declare each fact once. Derive every artifact from it.**

[![crates.io](https://img.shields.io/crates/v/fiducial.svg)](https://crates.io/crates/fiducial)
[![docs.rs](https://docs.rs/fiducial/badge.svg)](https://docs.rs/fiducial)

---

## What this is

The umbrella entry point for the [Fiducial](https://github.com/AleksaZCodes/fiducial)
platform — a system for building **cross-domain products** (electronics,
firmware, protocols, web, desktop, mechanical, simulation) where a fact is
declared in exactly one place and every artifact that depends on it is generated.

**This crate is a name claim and a signpost, not a facade.** Depending on it pulls
in nothing you need. Depend on the specific crate instead — that way a firmware
build never compiles a mesh exporter it will not use.

## Where to actually start

| You want to | Use |
|---|---|
| Scaffold and run a product | [`fiducial-cli`](../fiducial-cli) — the `fid` binary |
| Share logic across every runtime | [`fiducial-core`](../fiducial-core) |
| Talk to a device | [`fiducial-protocol`](../fiducial-protocol) |
| Carry units and tolerances | [`fiducial-quantity`](../fiducial-quantity) |
| Describe a board | [`fiducial-eda`](../fiducial-eda) |
| Generate an enclosure | [`fiducial-geometry`](../fiducial-geometry) + [`fiducial-mesh`](../fiducial-mesh) |
| Update firmware in the field | [`fiducial-ota`](../fiducial-ota) |
| Simulate | [`fiducial-sim`](../fiducial-sim) |

## Getting started

```sh
cargo install fiducial-cli
fid new my-product
cd my-product && fid dash
```

## The idea in one paragraph

In a cross-domain product the same fact is normally written down many times — a
pin assignment in the schematic, the firmware, the test rig and the docs; a board
dimension in the PCB tool, the enclosure model and the marketing render. Every
duplicate is a place where reality drifts from itself, and drift is discovered
late, in the field, expensively. So: one declaration, many derivations. The
declaration is typed, machine-readable, and lives in git. Everything downstream
is generated and never hand-edited.

Read [`MISSION.md`](../../MISSION.md) for the full reasoning.

## License

MIT

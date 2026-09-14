# fiducial-wasm

**The `no_std` spine, in a browser.**

[![crates.io](https://img.shields.io/crates/v/fiducial-wasm.svg)](https://crates.io/crates/fiducial-wasm)
[![docs.rs](https://docs.rs/fiducial-wasm/badge.svg)](https://docs.rs/fiducial-wasm)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

`wasm-bindgen` bindings that expose [`fiducial-core`](../fiducial-core) to
JavaScript — in the browser, at the edge, and in Cloudflare Workers. The same
compiled logic that runs on an RP2040 runs behind this wrapper, which is
[principle 2](../../MISSION.md) paying out: the behaviour was written once, at
the layer with the longest reach.

## The type boundary is generated, not written

Types that cross into TypeScript are emitted by `ts-rs` into
[`@fiducial/wasm-bridge`](../../packages/wasm-bridge):

```sh
cargo test --features ts -p fiducial-wasm
```

CI regenerates them and runs `git diff --exit-code`, so a Rust type change that
skipped regeneration fails the build. **Hand-writing a type that crosses this
boundary is guard-blocked** — it is a derived artifact, and derived artifacts are
never edited by hand.

## Build

```sh
wasm-pack build crates/fiducial-wasm --target web
```

## Use

```js
import init, { fiducialVersion, DeviceId } from "@fiducial/wasm-bridge";

await init();
console.log(fiducialVersion());
```

## Crate types

`cdylib` produces the `.wasm`; `rlib` keeps `cargo check` and host tests working,
so the crate is verified on the host as well as at its real target.

## License

MIT

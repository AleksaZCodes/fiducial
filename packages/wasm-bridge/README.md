# @fiducial/wasm-bridge

**Generated TypeScript types for the Rust ↔ JS boundary.**

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## ⚠️ This package is generated

`src/generated.ts` is emitted from the Rust types in
[`fiducial-wasm`](https://github.com/AleksaZCodes/fiducial/tree/main/crates/fiducial-wasm)
by [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs). **Do not hand-edit it** —
hand-editing a generated artifact is guard-blocked, and your change would be
erased by the next regeneration anyway.

To change a type here, change the Rust declaration and regenerate:

```sh
cargo test --features ts -p fiducial-wasm
```

CI runs that and then `git diff --exit-code`, so a Rust type change that skipped
regeneration fails the build rather than reaching `main` as a silent mismatch.

## Why this exists

A type that crosses a language boundary is the most reliable place for drift to
appear: two hand-written declarations of one shape, in two languages, with no
compiler that sees both. This package removes the second declaration. There is
one type, written in Rust, and this is its projection.

## Install

```sh
pnpm add @fiducial/wasm-bridge
```

## Use

```ts
import type { DeviceIdTs, PlatformVersion } from "@fiducial/wasm-bridge";

const id: DeviceIdTs = { bytes: [0xde, 0xad, 0xbe, 0xef, 0, 1, 2, 3] };
const version: PlatformVersion = { version: "0.1.0" };
```

## License

MIT

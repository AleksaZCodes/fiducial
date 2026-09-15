# fiducial-identity

**One identity model across users, devices and services — `no_std`.**

[![crates.io](https://img.shields.io/crates/v/fiducial-identity.svg)](https://crates.io/crates/fiducial-identity)

Before this crate the platform had three unrelated notions of "who":
`fiducial_core::DeviceId` at L0 (firmware, protocol, OTA), a user UUID inside
the `auth` adapter, and nothing at all for a service calling another service.
Each was correct alone and none could be compared to another — so *may this
actor do this to this thing?* had nowhere to live, and would have been
answered separately, and differently, at each level.

`Principal` is that missing type. It reuses `DeviceId` rather than minting a
second device identity, and is `no_std` so a device deciding "is this command
from my owner?" offline runs the same rule as the Worker at the edge.

```rust
use fiducial_identity::{can, Action, DeviceId, Grant, Principal, Resource, Role, UserId};

let owner = Principal::User(UserId::new([7; 16]));
let thermostat = Resource::Device(DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8]));
let grants = [Grant::new(owner, thermostat, Role::Owner)];

assert!(can(&owner, Action::Write, &thermostat, &grants));

// Deny is the default: a stranger gets nothing.
let stranger = Principal::User(UserId::new([9; 16]));
assert!(!can(&stranger, Action::Read, &thermostat, &grants));

// A device may read itself with no grant — provisioning depends on it
// being able to say "I am here, I am unclaimed" before anyone owns it.
let itself = Principal::Device(DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8]));
assert!(can(&itself, Action::Read, &thermostat, &[]));

// But not write itself: a compromised device must not be able to rewrite
// its own configuration and call it self-service.
assert!(!can(&itself, Action::Write, &thermostat, &[]));
```

## The rule

| | |
|---|---|
| **Deny by default** | an empty grant table permits nothing |
| **Unidentified is refused** | anonymous, or an all-zero sentinel id — an unprovisioned device must not authenticate as "device zero" |
| **A device may read itself** | with no grant; it may **not** write itself |
| **Platform scope** | a `Resource::Platform` grant covers every resource |
| **Roles are ordered** | `Viewer < Member < Admin < Owner`; a role permits every action at or below its level |

## Two implementations, one table

`can()` also exists in TypeScript (`@fiducial/identity`) for the Worker and
the app. An authorization rule is the worst possible place for drift — a
divergence does not look like a bug, it looks like access — so the decision
table is **generated from this crate** into `docs/identity/vectors.json` and
asserted by both suites:

```sh
FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --test vectors
```

CI runs it without the env var, so a rule change that skipped regeneration
fails the build. Same mechanism `docs/protocol/vectors.json` uses for the
wire format.

## Bridge from `auth`

`user_principal(&session.user.id)` turns the `auth` adapter contract's
session into a `Principal`. It takes the id rather than the session type so
this crate stays `no_std` and dependency-free.

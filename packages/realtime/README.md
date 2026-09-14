# @fiducial/realtime

**Three realtime contracts, testable without a server.**

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

Typed wrappers over the three realtime patterns a product actually needs:

| Contract | For |
|---|---|
| **Broadcast** | ephemeral fan-out — cursors, typing indicators, live events |
| **Presence** | who is here, and what state each client carries |
| **Postgres Changes** | `INSERT` / `UPDATE` / `DELETE` streamed from the database |

Each is an **adapter interface**, not a client. That is the
[principle 6](https://github.com/AleksaZCodes/fiducial/blob/main/MISSION.md)
shape: commit to the contract, stay permissive about the tool behind it. Supabase
satisfies these today; swapping the backend is an adapter, not a rewrite.

It is also why the 16 tests in this package run with **no live connection**. A
realtime layer you can only test against a running server is a realtime layer
that goes untested.

## Install

```sh
pnpm add @fiducial/realtime
```

## Use

```ts
import { createBroadcast, createPresence, createPostgresChanges } from "@fiducial/realtime";

// Broadcast — a typed event map, so a typo in an event name is a type error.
type Events = { cursor: { x: number; y: number } };
const channel = createBroadcast<Events>(adapter);

const off = channel.on("cursor", ({ x, y }) => draw(x, y));
channel.send("cursor", { x: 10, y: 20 });
off(); // unsubscribe

// Presence — typed per-client state.
type Player = { name: string; ready: boolean };
const presence = createPresence<Player>(adapter);
presence.onJoin(({ key, state }) => console.log(`${state.name} joined`));
presence.track({ name: "ada", ready: false });

// Postgres changes — narrowed per event type.
const rows = createPostgresChanges<Score>(adapter);
rows.onInsert((e) => append(e.new));
rows.onUpdate((e) => reconcile(e.old, e.new));
```

`presence.snapshot()` returns a **copy**, not a live reference — asserted by a
test, because handing out the internal map makes every consumer a potential
mutator.

## License

MIT

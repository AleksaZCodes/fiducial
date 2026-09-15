# @fiducial/identity

**One identity model across users, devices and services.**

Mirrors the Rust [`fiducial-identity`](../../crates/fiducial-identity) crate.
The Rust side runs in firmware, the desktop backend and the CLI; this side
runs in the Worker and the app. Both answer the same question — *may this
actor do this to this thing?* — and both are held to the same answer by
`docs/identity/vectors.json`, a decision table generated from the Rust crate
and replayed by this package's test suite.

```ts
import { can, principalFromSession } from "@fiducial/identity";
import { createAuth, CookieSessionContext } from "./adapters.generated.js";

const auth = createAuth(env, new CookieSessionContext(cookieJar));
const who = principalFromSession(await auth.getSession());

const thermostat = { kind: "device", id: "dadadadadadadada" } as const;
if (!can(who, "write", thermostat, grants)) {
  return new Response("forbidden", { status: 403 });
}
```

## The rule

| | |
|---|---|
| **Deny by default** | an empty grant table permits nothing |
| **Unidentified is refused** | anonymous, or an all-zero sentinel id |
| **A device may read itself** | with no grant; it may **not** write itself |
| **Platform scope** | a `platform` grant covers every resource |
| **Roles are ordered** | `viewer < member < admin < owner` |

## Ids are hex strings here

The Rust side holds ids as fixed-width bytes because it must run without an
allocator. This side holds them as lowercase hex, because that is what
crosses a wire, lands in a database column and appears in the vectors —
converting at each boundary instead would be three places to get it wrong.

`userIdFromUuid()` is the one conversion point: an auth vendor's UUID
subject in, a normalized `UserId` out, hyphens dropped and lowercased so the
same user in different casing is one identity rather than two.

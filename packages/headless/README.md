# @fiducial/headless

Framework-agnostic logic shared by every Fiducial surface — no DOM, no React, no
Svelte. The same `Result` flows through a web app, a Tauri desktop app, and a
worker without being rewritten per framework.

## Install

```sh
pnpm add @fiducial/headless
```

## `Result<T, E>`

An explicit success/failure value, so error handling shows up in the type rather
than depending on a caller anticipating a throw.

```ts
import { ok, err, isOk, isErr, unwrap, map, flatMap } from '@fiducial/headless'
import type { Result, Ok, Err } from '@fiducial/headless'

function parsePort(raw: string): Result<number, string> {
  const n = Number(raw)
  return Number.isInteger(n) && n > 0 ? ok(n) : err(`not a port: ${raw}`)
}

const r = parsePort(input)
if (isOk(r)) console.log(r.value)
else console.error(r.error)
```

`E` defaults to `Error`. `unwrap()` throws on an `Err`, so reach for `isOk()` on
any path where failure is expected. `map()` transforms a success and passes a
failure through untouched; `flatMap()` chains an operation that itself returns a
`Result`.

## `OfflineQueue<T>`

Accumulates actions while connectivity is unavailable and replays them in order
on reconnect.

```ts
import { OfflineQueue } from '@fiducial/headless'

const queue = new OfflineQueue<Command>({ maxRetries: 3 })
queue.enqueue('set-led', { on: true })

for (const action of queue.drain()) {
  await send(action.payload) // action.enqueuedAt is when it was queued
}
```

Each `QueuedAction<T>` carries `id`, `payload`, `enqueuedAt`, and `retries`.

The invariant worth knowing: actions replay at their **original `enqueuedAt`
timestamp, not at sync time**, so a burst of queued events keeps its true
ordering once the connection returns instead of collapsing into the moment of
reconnect.

`maxRetries` (default 3) bounds how many times an action is retried before it is
abandoned. API: `enqueue()`, `dequeue()`, `drain()`, `size`, `isEmpty`.

## License

MIT

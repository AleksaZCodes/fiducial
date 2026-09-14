# @fiducial/headless

## 0.3.0

### Minor Changes

- 9fbdd10: `OfflineQueue` gains durability, a backoff schedule, and `pending()`.
  
  Harvested from Ring of Pursuit (`docs/harvest/ring-of-pursuit.md`). The first
  pass took the invariant — actions replay at their original `enqueuedAt`
  timestamp, not at sync time — but left the durability behind. An in-memory
  offline queue loses its contents on a page reload, which is exactly what an
  offline user does.
  
  **New**
  
  - `QueueStorage<T>` — storage is an injected adapter. `memoryStorage()` is the
    default and what tests use; a browser product supplies an IndexedDB-backed one
    so the queue survives a reload. Deliberately synchronous: an async interface
    would make `enqueue` async, and an `enqueue` you can forget to await is a lost
    action.
  - `DEFAULT_BACKOFF_SCHEDULE_MS` — `[1000, 2000, 4000, 8000, 16000]`, roughly 31
    seconds total. Exported because it is a declared fact, not a magic number.
  - `backoffFor(action)` — the delay before the next attempt, or `undefined` when
    retries are exhausted and the caller should surface the failure instead of
    waiting.
  - `pending()` — the queue without draining it. Returns the actions rather than a
    count, so callers can filter by payload as well as count them.
  
  **Changed**
  
  - `maxRetries` now defaults to the length of `backoffScheduleMs` (5) rather than
    a hardcoded 3. A retry *count* and a retry *schedule* are two declarations of
    the same fact, and 3 retries against a 5-step schedule left the last two
    delays unreachable. An explicit `maxRetries` still overrides.

## 0.2.0

### Minor Changes

- 82d936a: Phase 7: L1 tokens and L2 headless packages
  
  `@fiducial/tokens` — L1 design token package. Exports typed color, spacing,
  typography, and border-radius tokens as `as const` objects. Ships a `./tailwind`
  entry that provides a Tailwind CSS preset extending the theme with all token sets.
  Both the tokens and the preset are framework-agnostic — no dependency on tailwindcss
  itself; consumers wire the preset object in their own config.
  
  `@fiducial/headless` — L2 headless utility package. Exports:
  - `Result<T, E>` type + `ok()`, `err()`, `isOk()`, `isErr()`, `unwrap()`, `map()`,
    `flatMap()` helpers — typed error-handling without exceptions.
  - `OfflineQueue<T>` — typed offline queue that accumulates actions when connectivity
    is unavailable and replays them in enqueuedAt order on reconnect. Key invariant from
    Ring of Pursuit: replay uses the original timestamp, not sync time, so event ordering
    stays coherent.

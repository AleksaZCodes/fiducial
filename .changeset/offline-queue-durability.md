---
"@fiducial/headless": minor
---

`OfflineQueue` gains durability, a backoff schedule, and `pending()`.

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

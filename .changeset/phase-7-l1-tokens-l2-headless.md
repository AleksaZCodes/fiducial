---
"@fiducial/tokens": minor
"@fiducial/headless": minor
---

Phase 7: L1 tokens and L2 headless packages

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

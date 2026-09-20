# @fiducial/tokens

## 0.3.0

### Minor Changes

- 4830c12: Four type roles, a corner strategy, and annotation ink.
  
  `fontRoles` / `fontDefaults` / `typeScale` split type into **display, body,
  script and mono**. The shadcn default pair — sans plus mono — cannot express the
  thing that most separates a designed page from a generated one: the display face
  is not the body face at a larger size. `typeScale` names ten steps so a page
  cannot invent an eleventh inline.
  
  `shape.ts` replaces the single `--radius` with a corner *strategy*
  (`round | chamfer | square`) plus a three-step scale (`panel` / `control` /
  `chip`). One variable can say how much; it can never say what kind, which is why
  every product built on the shadcn slot set has the same silhouette. Three steps
  rather than one because a chamfer must stay under half the element's height or
  the box degenerates into a lozenge.
  
  `--doodle-ink` and `--doodle-accent` join both themes — annotation is a second
  voice, and a second voice at full text contrast is a second headline.
  
  `generateThemeCss()` takes an options object (`fonts`, `shape`), emits the type
  roles as `--type-*` and maps them to `font-*` utilities in `@theme inline`, and
  forces `--radius` to 0 under a non-round strategy while still emitting the
  radius scale so components copied in from any shadcn-shaped registry resolve.
  
  Existing callers are unaffected: every option is optional and the previous
  output is a subset of the new one.

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

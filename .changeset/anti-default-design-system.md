---
"@fiducial/tokens": minor
---

Four type roles, a corner strategy, and annotation ink.

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

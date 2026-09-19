---
"fiducial-cli": minor
---

Components start at shadcn/ui, the brand declaration derives a module, and
locales name themselves.

**`[brand]` now derives `src/generated/brand.ts`.** Every other output of the
brand pipeline is read by a crawler — `robots.txt`, `sitemap.xml`, the manifest,
the JSON-LD — and none of them can be imported by the application. So the first
time app code needs the domain, for `metadataBase` or a canonical URL or an
absolute OG image, it gets retyped. It gets retyped into exactly one line, which
is what makes it dangerous: the duplicate is invisible in review, and it
surfaces the day the domain changes and the sitemap, the manifest and the legal
pages all move while the canonical URL quietly keeps pointing at the old host.
Nothing fails. The product just advertises the wrong origin. Add
`src/generated/brand.ts` to the brand pipeline's outputs to get it.

**The `design` capability ships adapted shadcn components.** `components.json`,
plus `button`, `card` and `dropdown-menu` already carrying the chamfer. The rule
is now written down: run `npx shadcn@latest add`, change the corner and the
surface, change nothing else. Focus rings, `aria-invalid`, disabled semantics,
icon sizing and `asChild` are the parts that are tedious to write and invisible
when wrong, and nobody re-derives that list from memory.

Under `chamfer` the adaptation is exactly three changes, because a clipped
element cannot cast an outer `box-shadow`: `rounded-*` becomes `.cham`, `bg-*`
becomes `--edge`/`--fill`, and shadows come off.

**Texture and depth.** `marks.css` gains the other half of the bevel — a
hairline inset highlight, since an inset shadow survives a `clip-path` where an
outer one does not — and a fixed SVG-filter grain layer that is off entirely
under `prefers-reduced-motion`. A product shipping only the weighted edge had
drawn half a bevel, which reads as a border that is wrong rather than a surface
that is raised.

**Locales declare themselves.** Each catalog now carries its own endonym and
flag region (`locale.name`, `locale.region`), and the capability ships a
`locale-picker` built by mapping over the locale list. The shape this replaces —
one key holding "the other language" — works for exactly two locales and is
silently wrong at three. `footer.madeWith` is seeded as the platform's own
attribution string, one value per locale like any other.

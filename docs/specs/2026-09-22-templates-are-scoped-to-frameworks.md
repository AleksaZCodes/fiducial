# A template can be scoped to another capability, and rasters are derived

**Date:** 2026-09-22
**Status:** accepted
**Supersedes:** nothing. Extends `2026-09-14-capability-taxonomy.md` with a
seventh kind.

---

## Context

Four problems arrived together from one product (`upoznaj-biznis`, a SvelteKit
app), and they turned out to be the same problem seen from four sides.

**1. `design` wrote React into a Svelte app.** The capability shipped
`button.tsx`, `card.tsx`, `dropdown-menu.tsx`, `doodle-arrows.tsx` and
`components.json` unconditionally, because a template's only metadata was its
path. A SvelteKit product got four files nothing in it could import — and
`fiducial.lock` recorded them as platform-owned, so `fid doctor` then
*required* their continued presence. Deleting dead code failed the gate. The
product's only way out was to restore them byte-for-byte and write down why.

**2. The locale picker existed only for React.** `i18n` shipped
`locale-picker.tsx`. A Svelte product could not use it, so the product
hand-wrote its own, and the platform gained a second implementation of the
same design with nothing keeping the two in step. The mobile menu was worse:
no template existed in any framework, so both products hand-built one and
neither was contributed back.

**3. Intl formatting was reinvented in the product.** `@fiducial/i18n` already
had memoized formatters, CLDR plural categories and money — and the product
wrote its own `Intl` wrappers anyway, because nothing installed a binding
between the generated catalog and that package.

**4. Rasters were a one-off script.** `pipelines/brand.toml` had carried a
note since it was written: raster favicons and OG images "need a rendering
step (fonts, rasterization) this pipeline does not carry yet." So the product
grew a script, committed the PNGs, and had no way to know whether they still
matched the SVGs they came from.

Underneath all four: **a capability could not say who a file was for.**

## Decision 1 — `for/<capability-id>/` scopes a template

A file under `for/web-next/` installs when `web-next` is installed and not
otherwise, at the path below `for/web-next/`. It is a seventh kind in the
taxonomy, and the layout carries the condition, exactly as `declarations/` and
`pipelines/` already carry theirs.

**Install order does not decide what a product has.** Installing a capability
also back-fills the `for/<it>/` templates of everything already installed, so
`i18n` then `web-svelte` and `web-svelte` then `i18n` produce the same
product. Without that pass the awkward order would leave a product with no
picker and nothing to report it — a file never written is not a file that is
stale.

Scoped templates are recorded in `fiducial.lock` like any other, so `fid
doctor` checks them and `fid upgrade` merges them.

### Why not a manifest key

`capability.toml` could have listed `[[templates]] path = …, requires = […]`.
The layout was chosen because it is the convention already in force: the
directory decides what a file is, and a capability with no manifest at all
still works. A manifest key would also have to be kept in step with the files
it names, which is the class of bug this platform exists to remove.

### What this does not solve

A capability still cannot declare an **npm dependency**. The Svelte picker
needs `bits-ui` and `country-flag-icons`; the React one needs
`@base-ui-components/react`. Both are declared in the *web* capability's
`package.json` template instead, which is honest about the primitive library
each framework uses but attaches the dependency to the wrong capability when
`i18n` is absent. A `requires_packages` declaration is the real fix and is not
built here.

## Decision 2 — a capability template may depend on a package, never on
another capability's copied file

`locale-picker.tsx` used to import `@/components/ui/dropdown-menu` and
`@/components/ui/button`. Those are files `design` *copies into* a product, so
the import resolved only where that copy had happened, and the failure was a
build error inside a file the product never wrote, blaming a capability that
had done nothing wrong.

Both pickers now use a headless primitive from npm — Base UI in React, bits-ui
in Svelte. bits-ui is what `shadcn-svelte` is itself built on, so a product
gets those components without the copy-in step, which is what "use proper
shadcn" means in a Svelte app.

## Decision 3 — components, not compositions

The platform ships `LocalePicker`, `MobileMenu` and `EntryCards`. It
deliberately does **not** ship a `SiteNav`.

That is where the line between capability template and product content falls.
A picker's behaviour, accessibility and locale mechanics are the same
everywhere and belong upstream. *Arranging* a wordmark, four links, a language
picker and a call to action is the decision each product makes differently:
one product's nav is `home`/`blog`/`press` with an RSVP anchor, another's is
`how`/`system`/`lineage` with a contact CTA. A `SiteNav` taking all of that as
props is a props-explosion with a component wrapped around it, and one that
hard-codes it is a component every product forks.

So the components take `links` and labels and own no words and no routes, and
composition stays in the product's layout.

## Decision 4 — rasters are derived, by a renderer written here

`pipelines/raster.toml` turns brand SVG marks into PNGs at derive time, gated
by `fid derive --check` like every other artifact. The outputs list is the
declaration — `<mark>-<size>.png` names its own source and size — and
`[raster]` in `fiducial.toml` adds only what a path cannot say: where the SVGs
live, and per-mark or per-output framing.

The renderer is ~400 lines of plain JavaScript in the capability, and that is
the surprising part. Two constraints rule out `sharp`, `resvg` and a headless
browser:

- **`fid derive --check` compares bytes**, so a renderer whose output tracks
  its own version turns the freshness gate into a coin toss, and a gate that
  fails at random is one people re-run instead of reading.
- **It has to run where nothing is installed**, or the pipeline passes on a
  laptop and fails in CI.

It renders paths, groups, flat fills and affine transforms. Everything else —
text, gradients, filters, arcs — is a **hard error, never an approximation**,
because a mark that renders *wrong* is the failure that ships: nobody reviews
the OG image of a page they did not change. Text especially: it needs a font,
no font version is pinned here, and two machines would produce different
pixels. The error says to convert text to paths.

**The determinism boundary is stated rather than assumed.** Geometry is plain
IEEE-754 and identical everywhere. The PNG container is zlib-compressed, and
zlib's output *can* move between Node majors — so the script decodes an
existing output and leaves it untouched when the pixels already match. A Node
upgrade therefore cannot fail the gate on a mark nobody edited, while a
changed mark still does. Verified both ways before this was written.

**`artbox` is the one option that is not obvious.** Centring a logo's full
drawing is usually wrong: a mark with an ascender rising out of the
letterforms sits visibly low, because the ascender counts as part of what is
being centred. Framing is per-output as well as per-mark, because what hangs
outside the box on a square avatar is simply cropped off a 1200×630 banner —
which the first run of this pipeline did, and the render was inspected rather
than assumed correct.

## Decision 5 — CSP belongs in `svelte.config.js`, and `fid doctor` looks there

The `web-svelte` capability shipped no `hooks.server.ts`, while `fid doctor`
requires one — so every SvelteKit product hand-wrote it, and the obvious
hand-written version is fatal.

SvelteKit boots the client from an inline `<script>` it generates. A CSP
written by hand cannot know that script's hash, so `script-src 'self'` blocks
it, the app never hydrates, and every interactive component silently stops
working **while the server-rendered HTML still looks perfect**. That shipped:
a countdown frozen at 00:00:00 and a language picker that would not open, on a
product whose build, typecheck and `fid doctor` were all green. It was found
by opening the deployed page in a real browser and reading the console — the
one check none of the gates performed.

So the capability now ships both files: `hooks.server.ts` with the required
headers, and `svelte.config.js` with `kit.csp` in hash mode, where SvelteKit
can derive the hash from the bytes it actually emitted. `fid doctor` reads
both, so declaring CSP where it works no longer fails the gate — the check
still fails when the header is absent everywhere, which is the failure it was
built for.

A regression test asserts all three: the CSP exists, it is not in the headers
file, and `fid doctor` accepts the result.

## Decision 6 — a shared component must not borrow a token across scales

Found by looking at the rendered page rather than at the test output, and
worth recording because the test could not have caught it.

The Svelte components style themselves from the design tokens with a fallback
on every one (`var(--border, currentColor)`), so they render plainly rather
than invisibly in a product without the `design` capability. That is right.
Reading `--radius-md` for a *large* surface was not: `upoznaj-biznis` declares
`--radius: 999px`, because it is a pill design system, and the press room's
logo frames rendered as **circles**.

The token was not wrong and the component was not wrong; the assumption that a
radius means the same thing at every size was. So:

- **Small controls** — buttons, menu rows — take the product's radius whole.
  A pill button in a pill design system is the intent.
- **Large surfaces** — cards, menus, image frames — take it capped:
  `min(var(--radius-md, 6px), 12px)`. A product with a 4px scale is unaffected;
  a product with a 999px scale gets a rounded rectangle instead of a circle.

The same pass fixed a second invisible-to-tests bug: `PressKit` used bare
`h1`/`h2`/`h3`, and a product whose stylesheet resets heading sizes — normal
when a design system supplies `.type-h2` classes instead — rendered the entire
press room at body size. The component now sets its own hierarchy in `em`, so
the scale is the component's and the typeface is the product's.

**The general rule:** a template that inherits a product's tokens inherits its
*decisions*, including ones it was not designed against. Check a shared
component against a product whose design system is nothing like the one you
wrote it in, and look at it.

## Decision 7 — the deploy capability describes a *shape*, not one artifact

`[deploy]` could only describe a standalone Worker at `apps/worker/`, so
installing the capability on a SvelteKit product wrote an
`apps/worker/wrangler.toml` describing nothing the product runs while the real
config stayed untracked and hand-edited. That is worse than not running it,
and it is why one product declared `[adapters] deploy = "cloudflare"` and then
deliberately did not install the capability — a declaration whose only honest
reading was "the platform cannot do this yet".

`[deploy] shape` names which artifact is being described:

- `worker` (the default) — `apps/worker/`, entrypoint `src/index.ts`, a source
  file this repository wrote.
- `sveltekit` — `apps/web` **is** the Worker under
  `@sveltejs/adapter-cloudflare`. The entrypoint and the static assets are both
  *build output*, and the config lives at the product root because that is
  where the adapter looks.

Three sub-decisions, each of which was a bug first:

**An unknown shape is an error, not a fallback.** `shape = "svelekit"` falling
back to `worker` generates a perfectly valid config for the wrong thing, which
is precisely the failure this field exists to end. It is refused by name.

**`main` is not seeded.** The capability used to seed `main = "src/index.ts"`
so the declaration would look complete. That value outlives the shape it was
true for: a product that later declares `shape = "sveltekit"` keeps pointing at
a source file it does not have, and the deploy succeeds onto nothing. `main`
now defaults to whatever the shape implies, so changing the shape moves the
entrypoint with it, and a product declares the field only to override it. The
general rule: **seed what cannot be derived, never what merely looks tidy** —
a seeded default is indistinguishable from an intentional one the moment
something else changes.

**The output path picks the format.** `.json`/`.jsonc` is written as JSONC,
anything else as TOML. Wrangler reads both, and which one a product wants is
not derivable from anything else: an app scaffolded by `create-cloudflare`
already has a `wrangler.jsonc`, and generating a second config in the other
format leaves two files disagreeing — the exact condition this capability
exists to end. The path was already saying *where*; saying *which format* costs
no new field.

`[deploy]` also gained `kv_namespaces`, `vars` and `observability`. KV has no
`[adapters]` contract to derive a binding from, and the id is load-bearing in
the worst way: point a deploy at a different namespace and the data is silently
gone rather than missing.

**How this was checked.** Field by field against a live, hand-written
`wrangler.jsonc` deploying a real SvelteKit Worker: `name`, `main`,
`compatibility_date`, `compatibility_flags`, `observability`, `assets` and
`kv_namespaces` all match what the generator emits. The one difference is
`vars`, where the generator additionally emits `FIDUCIAL_PRODUCT` — an existing
convention of the Worker shape, not a new behaviour. That product is **not**
migrated onto the capability here: doing so would rewrite a live Worker name,
KV id and domain binding days before the thing it serves happens, and the
generator being provably correct is not a reason to do it today.

## Consequences

- A capability can ship the same idea for two frameworks without either
  product carrying the other's files. `i18n`, `content`, `design`, `press`
  now do.
- The `design` bug is fixed at the source. A product working around it should
  delete the restored stubs and re-run `fid derive`.
- `[brand] platform_credit` is opt-in and **false by default** — a credit line
  is a sentence printed under someone else's brand, to their readers. It is
  seeded explicitly rather than left to a struct default, because a flag
  nobody can see is a flag nobody uses.
- A capability still cannot declare an npm dependency; see Decision 1.
- The `deploy` capability now covers the SvelteKit shape, so the routing work
  sequenced behind it is unblocked — but nothing live is migrated onto it yet.
- Domain-per-locale routing is still not addressed. `locale-href.ts` ships the
  two builders that work today (`?lang=` and `/en/…`); choosing between them
  is a routing decision a component must not make, and a *third* strategy is
  sequenced behind the `deploy` capability's gap.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

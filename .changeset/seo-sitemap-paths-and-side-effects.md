---
"@fiducial/cli": minor
---

Make the `seo` capability work outside the product it was written against.

It shipped with no test, and three defects were sitting in it — each one
invisible on Next.js and fatal anywhere else.

**Output paths are the pipeline's declaration.** The deriver hard-coded
`apps/web/public/…` while `pipelines/seo.toml` also declared it. The documented
one-line change for a SvelteKit product — `public` → `static` — was therefore
necessary and insufficient: the pipeline guarded a path the script never wrote,
so `fid derive --check` failed on a *missing* artifact and pointed at neither
the cause nor the fix. The script now reads its three output paths out of its
own pipeline, so that edit is enough.

**No undeclared writes.** It imported `parseToml` from `derive-content.mjs`, a
pipeline executor with no main-module guard, so running the `seo` pipeline
re-derived the entire content model as an import side effect — writing
`generated/content.ts`, which `seo` does not declare as an output. Nothing ever
failed, because content's own pipeline writes the same bytes; the bug was a file
changing under a pipeline that never claimed it. It now imports the parser from
`toml-lite.mjs`, which is a parser and nothing else.

**The sitemap-collision guard matches a file name.** It compared against the
literal `apps/web/public/sitemap.xml`, so it went quiet in exactly the products
whose paths differ — the ones that had to edit the pipeline, and therefore the
population most likely to have left both `brand` and `seo` owning the file. Two
pipelines writing one artifact is a race settled silently by pipeline name
order.

**`[seo] pages` carries its own message keys.** The script held a table of four
paths — `/`, `/press`, `/blog`, `/credits` — with one product's key spellings
baked in, and any other page set hit "no title is declared for it". That named
the wrong problem: the page had a title, under a key the script had never heard
of. A page may now declare where its copy lives:

```toml
[seo]
pages = [
    { path = "/", title = "meta.title", description = "meta.description" },
    { path = "/workshop", title = "workshop.heading", description = "workshop.blurb" },
]
```

A bare path still resolves through the conventional keys, and an unrecognised
one now prints the declaration to write instead of a misleading diagnosis. This
also removes the pressure to satisfy a hard-coded key by writing a string a
second time, which is the duplicate the platform exists to prevent.

`seo` now declares `brand` among its requirements, which it always needed: every
URL it emits starts with `[brand] domain`.

Covered by `tests/seo_pipeline.rs` — nine end-to-end cases, one per defect.

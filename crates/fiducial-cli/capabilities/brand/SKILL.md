# fiducial:brand — Brand Skill

This product has the `brand` capability. **One declaration, many derivations**
— `MISSION.md` principle 1, applied to the facts a product's identity is made
of: legal name, trading name, domain, contact email, and the two colours
everything else is generated in.

## The loop

```
[brand] in fiducial.toml      ← you edit this
  ↓  fid derive                (pipeline: brand — executor fid-brand)
public/robots.txt
public/sitemap.xml
public/site.webmanifest
public/favicon.svg
public/organization.jsonld    ← generated: never hand-edit these
  ↓
fid derive --check            ← fails if any is missing or stale
```

## The declaration

```toml
[brand]
legal_name        = "Example LLC"
trading_name      = "Example"
domain            = "example.com"
contact_email     = "hello@example.com"
primary_color     = "#0EA5E9"
background_color  = "#0B1120"
```

Installing this capability seeds all six fields with placeholder text so the
pipeline does not fail on the very first `fid derive` — **replace every one**
before deriving for real. `fid derive` refuses to run against a domain,
name or contact left as the placeholder's actual text only by accident of
matching — it validates that every field is *present*, not that it is yours,
so read what you write here once.

`primary_color` and `background_color` are `#RRGGBB` hex. They are the only
two colours a favicon and a web manifest need; a full palette is
`@fiducial/tokens`, which this declaration is the first step toward feeding.

## What each output is

| File | Is |
|---|---|
| `robots.txt` | Allows every crawler, points at the sitemap |
| `sitemap.xml` | One `<url>` for the domain root today — add more by hand until routes are their own declaration |
| `site.webmanifest` | App name and theme colours, pointing at the generated favicon |
| `favicon.svg` | A rounded square in `primary_color`, with the trading name's initials in `background_color` — vector, so one file covers every size a browser asks for |
| `organization.jsonld` | A `schema.org` `Organization` record — wire it into your page with `<script type="application/ld+json">` reading this file's content |

**Never hand-edit any of the five.** They are guard-blocked and rewritten on
every `fid derive`. Change `[brand]`, run `fid derive`.

## Rules

**1 · A brand fact is declared once, here.** A hex colour typed a second time
into a component is the same drift `MISSION.md` principle 1 exists to forbid —
read it from `@fiducial/tokens` (fed by this declaration) instead.

**2 · `sitemap.xml` needs your routes added by hand for now.** This pipeline
derives the one URL every site has; a router-fed derivation is future work,
not something to fake by editing the generated file — add real `<url>` entries
to `pipelines/brand.toml`'s neighbourhood in your own build step until then, or
extend the pipeline.

**3 · The output paths assume Next.js's `public/`.** A SvelteKit product edits
`pipelines/brand.toml`'s `outputs` to point at `apps/web/static/…` instead —
the declaration does not move, only where the derivation lands.

## What this capability does not do yet

Raster favicons (`.ico`, PNG sizes), OG/Twitter card images, a press kit,
social post templates, and email themes rendered in the product's theme are
all named in `ROADMAP.md` § Brand and are not derived here. Each needs a
rendering step — fonts, rasterization — this pipeline does not carry. Adding
one is a new output name in `pipelines/brand.toml` and a new branch in
`fid-brand`, not a redesign of the declaration.

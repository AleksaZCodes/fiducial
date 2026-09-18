/**
 * Builds the design-system gallery in `design-system/`.
 *
 *   node scripts/build-design-system.mjs
 *
 * ## Why this is generated and not written
 *
 * The tokens are NOT retyped here — they are read from the app's global
 * stylesheet, which stays the single declaration. That is the whole reason the
 * gallery is generated: a hand-built swatch page drifts from the product within
 * a week and then actively lies about what the system is.
 *
 * If a swatch here looks wrong, the token is wrong. Fix it upstream.
 *
 * ## What you are expected to edit
 *
 * `CONFIG` below, and the `pages` at the bottom. Everything between them is
 * plumbing. Adding a page is adding an entry to `pages`.
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const ROOT = join(import.meta.dirname, "..");

const CONFIG = {
  /** The derived token stylesheet — colours, type roles, shape, named steps. */
  tokens: "apps/web/src/app/tokens.css",
  /** The mechanism layer. Inlined into the previews, which have no build step. */
  marks: "apps/web/src/app/marks.css",
  /**
   * The app's own stylesheet, for anything it declares that is not a token —
   * composed surfaces, a glow, a veil. Its `@import`s are stripped; its base
   * layer is harmless in a preview and its extras are the point.
   */
  app: "apps/web/src/app/globals.css",
  /** Where the gallery lands. */
  out: "design-system",
  /**
   * Fonts for the preview pages only.
   *
   * The app self-hosts its faces (next/font, @font-face); these standalone HTML
   * files have no bundler, so they pull the same families from a CDN. Keep this
   * list in step with the `fid:type` block in `design-system.md` — if a preview
   * shows a face the product does not load, the gallery is lying in the other
   * direction.
   */
  fontHref:
    "https://fonts.googleapis.com/css2" +
    "?family=Chakra+Petch:wght@500;600;700" +
    "&family=Source+Serif+4:opsz,wght@8..60,400;8..60,600" +
    "&family=Caveat:wght@500;600" +
    "&family=JetBrains+Mono:wght@400;500" +
    "&display=swap",
};

const read = (rel) => readFileSync(join(ROOT, rel), "utf8");

// The previews are plain HTML with no build step, so every `@import` has to be
// resolved here: Tailwind is dropped (the handful of utilities the previews use
// are declared below) and the rest is inlined in cascade order.
const stripImports = (css) => css.replace(/^\s*@import\s+[^;]+;\s*$/gm, "");

const tokens = stripImports(read(CONFIG.tokens));
const appExtras = stripImports(read(CONFIG.app));

const PREVIEW_CSS = `
${tokens}

${read(CONFIG.marks)}

${appExtras}

/* ── preview-only scaffolding (not part of the design system) ─────────────── */
body { font-family: var(--type-body); padding: 2rem; background: var(--background); color: var(--foreground); }
.row { display: flex; flex-wrap: wrap; gap: .75rem; align-items: center; }
.stack { display: grid; gap: 1.5rem; }
.grid { display: grid; gap: 1rem; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); }
.label { font-family: var(--type-display); font-size: .7rem; text-transform: uppercase; letter-spacing: .14em; color: var(--muted-foreground); }
.swatch { height: 72px; }
.btn { padding: .8rem 1.4rem; font-family: var(--type-body); font-size: .875rem; font-weight: 500; cursor: pointer; border: 0; color: inherit; }
.btn-primary { --edge: var(--primary); --fill: var(--primary); color: var(--primary-foreground); }
.btn-outline { --fill: transparent; }
.chip { font-family: var(--type-body); font-size: .75rem; padding: .35rem .8rem; color: var(--muted-foreground); display: inline-block; }
.card { padding: 1.4rem; }
h1, h2, h3 { margin: 0; }
p { margin: 0; }
code { font-family: var(--type-mono); font-size: .85em; }
figure { margin: 0; }
`;

const page = (title, group, subtitle, body) => `<!-- @dsCard group="${group}" -->
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="${CONFIG.fontHref}" rel="stylesheet">
<style>${PREVIEW_CSS}</style>
</head>
<body>
<div class="stack">
  <div>
    <p class="label">${group}</p>
    <h2 class="type-h2" style="margin-top:.35rem">${title}</h2>
    <p class="type-small" style="color:var(--muted-foreground);margin-top:.5rem;max-width:68ch">${subtitle}</p>
  </div>
  ${body}
</div>
</body>
</html>`;

/** One colour, its value, and what it is for. A swatch without a role is decoration. */
const swatch = (name, varName, note) => `
  <figure>
    <div class="cham swatch" style="--edge: var(${varName}); --fill: var(${varName})"></div>
    <figcaption>
      <p class="type-small" style="margin-top:.45rem;font-weight:600">${name}</p>
      <p class="type-small" style="color:var(--muted-foreground);font-size:.72rem">${note}</p>
    </figcaption>
  </figure>`;

/** One step of the named scale, rendered at its own size. */
const step = (cls, spec, sample) => `
  <div>
    <p class="label">${cls} · ${spec}</p>
    <p class="${cls}" style="margin-top:.4rem">${sample}</p>
  </div>`;

// ── Pages ────────────────────────────────────────────────────────────────────
// Edit freely. Each entry is one card in the gallery.

const pages = {
  "foundations/color.html": page(
    "Colour", "Foundations",
    "Every value is a token. Nothing here is written at a call site, and a colour that is not on this page does not exist in the product.",
    `<div class="grid">
      ${swatch("Primary", "--primary", "primary actions and links")}
      ${swatch("Accent", "--accent", "highlights — never white text on it")}
      ${swatch("Background", "--background", "page ground")}
      ${swatch("Foreground", "--foreground", "body text")}
      ${swatch("Muted", "--muted", "section bands, inset surfaces")}
      ${swatch("Muted foreground", "--muted-foreground", "secondary text and captions — the pair most likely to fail AA")}
      ${swatch("Border", "--border", "card edges and rules")}
      ${swatch("Destructive", "--destructive", "errors only")}
    </div>
    <div class="cham card">
      <p class="label">Annotation ink</p>
      <div class="row" style="margin-top:.8rem">
        <div style="width:44px;height:44px;background:var(--doodle-ink)"></div>
        <div style="width:44px;height:44px;background:var(--doodle-accent)"></div>
        <p class="type-small" style="color:var(--muted-foreground);max-width:52ch">Marks stroke in <code>--doodle-ink</code>, deliberately quieter than body text. <code>--doodle-accent</code> is the one that gets to shout — at most one mark per viewport.</p>
      </div>
    </div>`),

  "foundations/type.html": page(
    "Typography", "Foundations",
    "Four roles, ten steps, nothing outside them. The display face is not the body face at a larger size — that difference is most of what separates a designed page from a generated one.",
    `<div class="stack">
      ${step("type-display", "display · 3.75rem / 1.04 / 600", "The display face")}
      ${step("type-h1", "display · 2.75rem / 1.1 / 600", "A section heading")}
      ${step("type-h2", "display · 2rem / 1.18 / 600", "A subsection heading")}
      ${step("type-h3", "display · 1.25rem / 1.3 / 600", "A card title")}
      ${step("type-subhead", "body · 1.1875rem / 1.6 / 400", "The sentence under a heading, set in the body face so the eye changes voice rather than just size.")}
      ${step("type-body", "body · 1rem / 1.65 / 400", "Running text. Measure caps at sixty-eight characters, because a wider line is not a longer line — it is a line nobody finishes.")}
      ${step("type-small", "body · 0.875rem / 1.6 / 400", "Captions, secondary detail, and the small print that still has to be read.")}
      ${step("type-eyebrow", "display · 0.75rem / 600 / 0.18em", "Section label")}
      ${step("doodle-note", "script · 1.25rem / 1.25 / 500", "…and the hand in the margin")}
      ${step("type-mono", "mono · 0.8125rem / 1.5 / 400", "01 · 02 · 03 · 04 — tabular figures on by default")}
    </div>`),

  "foundations/shape-space.html": page(
    "Shape and space", "Foundations",
    "Corners are a strategy, not a number. One scalar can say how much; it can never say what kind, which is why every product built on a single --radius has the same silhouette.",
    `<div class="grid">
      <figure><div class="cham" style="height:80px"></div><figcaption class="type-small" style="margin-top:.5rem">--corner-panel · cards and panels</figcaption></figure>
      <figure><div class="cham-sm" style="height:80px"></div><figcaption class="type-small" style="margin-top:.5rem">--corner-control · buttons, nav</figcaption></figure>
      <figure><div class="cham-xs" style="height:80px"></div><figcaption class="type-small" style="margin-top:.5rem">--corner-chip · chips and pills</figcaption></figure>
      <figure><div class="cham-b" style="height:80px"></div><figcaption class="type-small" style="margin-top:.5rem">bottom-only · the bar shape</figcaption></figure>
      <figure><div class="cham" style="height:80px;--edge:var(--primary);--fill:var(--primary)"></div><figcaption class="type-small" style="margin-top:.5rem">solid · edge and fill agree</figcaption></figure>
    </div>
    <div class="cham card">
      <p class="label">Why three sizes and not one scalar</p>
      <p class="type-small" style="color:var(--muted-foreground);margin-top:.6rem;max-width:68ch">A chamfer must stay under half the element's height, or the two cuts on one edge meet and the box degenerates into a lozenge. A chip given the panel corner is not slightly wrong — it is a different shape. That constraint is the reason the scale is tied to element classes rather than to a size ramp.</p>
      <p class="type-small" style="color:var(--muted-foreground);margin-top:.8rem;max-width:68ch">The border is not a <code>border</code>: <code>clip-path</code> cuts a stroke off along the diagonal and leaves the corners bare. So the element's own background paints the edge and an inset pseudo-element paints the fill. Callers set <code>--edge</code> and <code>--fill</code>.</p>
    </div>`),

  "components/actions.html": page(
    "Buttons and status", "Components",
    "One primary action per view. The status pill is the honest one — it says what is actually true about a thing, and the design system exists partly to keep it that way.",
    `<div class="stack">
      <div><p class="label">Buttons</p><div class="row" style="margin-top:.6rem">
        <button type="button" class="cham-sm btn btn-primary">Primary action</button>
        <button type="button" class="cham-sm btn btn-outline">Secondary</button>
        <button type="button" class="cham-xs btn" style="--fill: transparent; --edge: transparent">Ghost</button>
      </div></div>
      <div><p class="label">Status pills</p><div class="row" style="margin-top:.6rem">
        <span class="cham-xs chip">Designed</span><span class="cham-xs chip">Planned</span><span class="cham-xs chip">Roadmap</span>
      </div></div>
      <div><p class="label">Live badge</p><div class="row" style="margin-top:.6rem">
        <span class="cham-xs chip" style="display:inline-flex;align-items:center;gap:.45rem;padding:.3rem .7rem">
          <span style="width:6px;height:6px;border-radius:999px;background:var(--accent);display:inline-block"></span>In development
        </span>
      </div></div>
    </div>`),

  "components/marks.html": page(
    "Annotation marks", "Components",
    "The one device here a layout algorithm cannot imitate: a mark says someone looked at this and pointed. Which is exactly why it is rationed — at most two per viewport, at most one of them in the accent ink.",
    `<div class="grid">
      <figure>
        <svg viewBox="0 0 120 44" fill="none" stroke="var(--doodle-ink)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="width:100%;overflow:visible">
          <path d="M4 11 C 32 3, 74 7, 103 27" pathLength="1"/><path d="M 87 20 L 105 29 L 94 12" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Arrow · curve</figcaption>
      </figure>
      <figure>
        <svg viewBox="0 0 120 70" fill="none" stroke="var(--doodle-ink)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="width:100%;overflow:visible">
          <path d="M5 60 C 17 24, 53 5, 70 21 C 81 32, 62 47, 51 36 C 39 24, 63 9, 89 16 C 101 19, 108 26, 112 35" pathLength="1"/><path d="M 103 25 L 113 37 L 98 40" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Arrow · loop</figcaption>
      </figure>
      <figure>
        <svg viewBox="0 0 200 90" fill="none" stroke="var(--doodle-accent)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="width:100%;overflow:visible">
          <path d="M150 12 C 109 1, 39 5, 19 30 C 1 52, 30 78, 85 82 C 141 86, 187 69, 188 43 C 189 21, 159 9, 127 8" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Circle · overshoots its own start, on purpose</figcaption>
      </figure>
      <figure>
        <svg viewBox="0 0 120 12" fill="none" stroke="var(--doodle-ink)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="width:100%;overflow:visible">
          <path d="M2 7 C 12 2, 20 10, 30 6 C 40 2, 48 10, 58 6 C 68 2, 76 10, 86 6 C 96 2, 106 10, 118 5" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Underline · squiggle</figcaption>
      </figure>
      <figure>
        <svg viewBox="0 0 44 36" fill="none" stroke="var(--doodle-ink)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="height:44px;overflow:visible">
          <path d="M3 18 C 9 22, 13 27, 17 33 C 23 20, 31 9, 42 2" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Check · confirmed</figcaption>
      </figure>
      <figure>
        <svg viewBox="0 0 36 36" fill="none" stroke="var(--doodle-ink)" stroke-width="var(--doodle-stroke)" stroke-linecap="round" stroke-linejoin="round" style="height:44px;overflow:visible">
          <path d="M4 4 C 13 13, 22 23, 32 33" pathLength="1"/><path d="M32 4 C 23 13, 13 23, 4 33" pathLength="1"/>
        </svg>
        <figcaption class="type-small" style="margin-top:.5rem">Cross · dismissed</figcaption>
      </figure>
    </div>
    <div class="cham card">
      <p class="label">The rules</p>
      <p class="type-small" style="color:var(--muted-foreground);margin-top:.6rem;max-width:68ch">Marks are <code>aria-hidden</code>: one that points at text is emphasis, not information, and the text has to carry the meaning alone. They stroke in <code>--doodle-ink</code>, never <code>--foreground</code> — an annotation at full text contrast is a second headline. The script face appears in <code>doodle-note</code> and nowhere else.</p>
      <p style="margin-top:1rem"><span class="doodle-note" style="transform:rotate(-3deg);color:var(--doodle-ink)">…like this, and only like this</span></p>
    </div>`),

  "components/cards.html": page(
    "Cards", "Components",
    "Flat, 2px edge, no shadow and no hover lift. Under the chamfer strategy a clip-path element cannot cast an outer shadow anyway — so elevation is carried by the edge, which is the more honest cue.",
    `<div class="grid">
      <div class="cham card"><div class="row" style="justify-content:space-between;align-items:flex-start"><h3 class="type-h3">Card title</h3><span class="cham-xs chip">Status</span></div><p class="type-small" style="color:var(--muted-foreground);margin-top:.7rem">A status label is load-bearing: it says what is actually true about the thing, and it is the first place a design system starts telling comfortable lies.</p></div>
      <div class="cham card"><div class="row" style="justify-content:space-between;align-items:flex-start"><h3 class="type-h3">Second card</h3><span class="cham-xs chip">Status</span></div><p class="type-small" style="color:var(--muted-foreground);margin-top:.7rem">Cards sit in a grid with a 1rem gutter and do not move when pointed at.</p></div>
    </div>`),

  "components/navigation.html": page(
    "Navigation", "Components",
    "Text links, no icons. Sticky, frosted, and chamfered along the bottom edge only — so the bar reads as hanging from the top of the viewport rather than floating near it.",
    `<div style="border:1px solid var(--border);overflow:hidden">
      <div style="position:relative;height:210px;background:var(--muted)">
        <div class="frosted cham-b" style="--edge: var(--border);display:flex;align-items:center;gap:1.5rem;padding:0 1.25rem;height:64px">
          <strong class="type-h3" style="font-size:.95rem">Product</strong>
          <span class="type-small" style="margin-left:auto;display:flex;gap:1.5rem;color:var(--muted-foreground)">
            <span>Section</span><span>Section</span><span>Section</span>
          </span>
          <button type="button" class="cham-sm btn btn-primary" style="padding:.5rem .9rem">Contact</button>
        </div>
        <p class="type-small" style="padding:1.5rem 1.25rem;color:var(--muted-foreground)">A clip-path element cannot cast an outer shadow, so the scroll cue is the 2px edge itself — transparent at rest, drawn once there is page behind the bar.</p>
      </div>
    </div>`),
};

for (const [path, html] of Object.entries(pages)) {
  const full = join(ROOT, CONFIG.out, path);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, html);
  console.log(`wrote ${CONFIG.out}/${path}`);
}

writeFileSync(join(ROOT, CONFIG.out, "tokens.css"), tokens);
console.log(`wrote ${CONFIG.out}/tokens.css (copied from ${CONFIG.tokens})`);

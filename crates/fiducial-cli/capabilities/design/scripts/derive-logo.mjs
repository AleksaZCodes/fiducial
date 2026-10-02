// The two logo primitives → one generated module everything else reads.
//
// `brand/icon.svg` and `brand/wordmark.svg` are the source of truth for the
// mark. This script extracts their geometry into
// `apps/web/src/generated/logo.ts` so React renders the same paths the SVG
// files carry, rather than a second copy typed into a component.
//
// It exists because the copy was real and it was wrong: a favicon held the
// mark from before it was redrawn, and its own comment said the duplication
// was cheaper than deriving it. It was not — the site shipped the old mark at
// 16px while the wordmark on the page showed the new one, and nothing could
// have told anyone.
//
// The primitives name their parts with `data-layer` groups:
//
//   brand/icon.svg      `mark`     the mark — paths, filled evenodd
//   brand/wordmark.svg  `letters`  the name — paths, in the ink colour
//                       `accent`   optional — paths or polygons in the
//                                  primary colour beside the letters
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), "utf8");

/**
 * Reject a primitive that is not well-formed XML.
 *
 * A `.svg` file is served as `image/svg+xml` and parsed strictly, so a browser
 * will refuse to render it — silently, as a broken image — where an HTML parser
 * would have shrugged. The failure that prompted this check: a `--` inside an
 * XML comment, which is illegal and is exactly what you get from writing a CLI
 * flag like `derive --check` into a comment explaining the file.
 *
 * Only the comment-level trap is checked here, because it is the one that a
 * human writing documentation will hit again and again.
 */
function assertWellFormed(name, svg) {
  for (const m of svg.matchAll(/<!--([\s\S]*?)-->/g)) {
    if (m[1].includes("--")) {
      throw new Error(
        `${name}: an XML comment contains "--", which is not legal and makes ` +
          `the file unrenderable. Rewrite the comment (a CLI flag like ` +
          `\`--check\` is the usual culprit).`,
      );
    }
  }
  if (!/<svg[\s>]/.test(svg)) throw new Error(`${name}: no <svg> element`);
}

/** Pull `<path d="…">`/`<polygon points="…">` and the viewBox out of an SVG. */
function parse(svg) {
  const viewBox = svg.match(/viewBox="([^"]+)"/)?.[1];
  if (!viewBox) throw new Error("no viewBox");
  const layer = (id) => {
    const g = svg.match(new RegExp(`<g[^>]*data-layer="${id}"[^>]*>([\\s\\S]*?)</g>`))?.[1];
    if (g === undefined) return null;
    return {
      paths: [...g.matchAll(/<path[^>]*\sd="([^"]+)"/g)].map((m) => m[1]),
      polygons: [...g.matchAll(/<polygon[^>]*\spoints="([^"]+)"/g)].map((m) => m[1]),
    };
  };
  return { viewBox, layer };
}

/**
 * The mark's geometry, read straight from the two primitives.
 *
 * Exported so that anything else drawing this logo calls it instead of reading
 * a derived file. That matters for ordering: `fid derive` does not run
 * pipelines in dependency order — it has no `needs` field, and the order turned
 * out to be neither alphabetical nor declared — so a script that reads
 * `generated/logo.json` may run before the pipeline that writes it. Reading the
 * primitives has no such hazard, because they are source, not derived.
 */
export function logoGeometry() {
  const iconSvg = read("brand/icon.svg");
  const wordSvg = read("brand/wordmark.svg");
  assertWellFormed("brand/icon.svg", iconSvg);
  assertWellFormed("brand/wordmark.svg", wordSvg);
  const icon = parse(iconSvg);
  const word = parse(wordSvg);

  const mark = icon.layer("mark");
  const letters = word.layer("letters");
  const accent = word.layer("accent") ?? { paths: [], polygons: [] };
  if (!mark) throw new Error('brand/icon.svg has no <g data-layer="mark"> group');
  if (!letters) throw new Error('brand/wordmark.svg has no <g data-layer="letters"> group');
  return {
    iconViewBox: icon.viewBox,
    markViewBox: markBox(mark.paths.join(" ")),
    wordmarkViewBox: word.viewBox,
    MARK: mark.paths.join(" "),
    LETTERS: letters.paths,
    ACCENT: accent,
  };
}

/** The accent, as SVG elements in one colour. */
export function accentSvg(accent, fill) {
  return (
    `<g fill="${fill}">` +
    accent.paths.map((d) => `<path d="${d}"/>`).join("") +
    accent.polygons.map((pts) => `<polygon points="${pts}"/>`).join("") +
    "</g>"
  );
}

const g = logoGeometry();

// A square box tight to the mark and centred on its own width, for the places
// the mark is used bare (an app icon slot, a bullet, a favicon caption). Derived
// from the path rather than typed, so it follows the drawing if the mark moves.
function markBox(d) {
  const n = d.match(/-?\d+(?:\.\d+)?/g).map(Number);
  const xs = n.filter((_, i) => i % 2 === 0);
  const ys = n.filter((_, i) => i % 2 === 1);
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  const [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  const size = Math.max(x1 - x0, y1 - y0);
  const cx = (x0 + x1) / 2;
  return `${+(cx - size / 2).toFixed(4)} ${y0} ${size} ${size}`;
}

const lines = [
  "// Generated by `node scripts/derive-logo.mjs` from the two logo primitives.",
  "// Do not edit — edit brand/icon.svg or brand/wordmark.svg and re-derive.",
  "//",
  "// There are exactly two logo primitives and everything else is a view of",
  "// them: the favicon, the app icon, the wordmark in the nav and the footer,",
  "// the mark on a slide. A component that types out its own path data is a",
  "// third logo that nobody will remember to update.",
  "",
  `export const iconViewBox = ${JSON.stringify(g.iconViewBox)} as const`,
  "",
  "/** Square and tight to the mark, in its own coordinates. */",
  `export const markViewBox = ${JSON.stringify(g.markViewBox)} as const`,
  `export const wordmarkViewBox = ${JSON.stringify(g.wordmarkViewBox)} as const`,
  "",
  "/** The mark. Fill-rule evenodd, in `--primary`. */",
  `export const MARK = ${JSON.stringify(g.MARK)}`,
  "",
  "/** The wordmark's letters, in the ink colour. */",
  `export const LETTERS = ${JSON.stringify(g.LETTERS, null, 2)} as const`,
  "",
  "/** The wordmark's accent, if it has one, in `--primary`. */",
  `export const ACCENT = ${JSON.stringify(g.ACCENT, null, 2)} as const`,
  "",
];
// JSON as well as TS. The TS module is for the React components; the JSON is
// for the node scripts that also draw this mark — the design gallery, and
// anything else that has to render it outside a bundler. Without it a script
// keeps its own copy, which is precisely the failure this pipeline exists to
// remove, and it had already happened twice.
const json = g;
const jsonOut = "apps/web/src/generated/logo.json";
mkdirSync(dirname(new URL(`../${jsonOut}`, import.meta.url).pathname), { recursive: true });
writeFileSync(new URL(`../${jsonOut}`, import.meta.url), `${JSON.stringify(json, null, 2)}\n`);
console.log(`wrote ${jsonOut}`);

// The wordmark, published for download.
//
// The press kit needs a file a journalist can save, and `brand/icon.svg`
// already reaches `public/favicon.svg` through the brand pipeline. This puts
// the other primitive beside it. Copied from the primitive rather than
// re-emitted, so there is still exactly one drawing of the mark.
const pub = "apps/web/public/wordmark.svg";
mkdirSync(dirname(new URL(`../${pub}`, import.meta.url).pathname), { recursive: true });
writeFileSync(new URL(`../${pub}`, import.meta.url), read("brand/wordmark.svg"));
console.log(`wrote ${pub}`);

// ── The asset set ────────────────────────────────────────────────────────────
// What anybody outside this project is given: the wordmark and the mark, each
// in a light-background and a dark-background variant, on **transparent**.
//
// Three properties, each for a reason someone hit:
//
//   · **Transparent.** An asset with a baked background can only be used on
//     that background. The first thing a journalist or a partner does is put
//     the mark on their own surface.
//   · **Two variants, not one plus an instruction.** The mark at `--primary`
//     is legible on light and muddy on dark, and the letters invert outright.
//     "Use the other colour" is a rule nobody reads; two files is not.
//   · **Colours from the tokens, not typed here.** The light variant is the
//     light theme's `--primary` and `--foreground`; the dark variant is the
//     dark theme's. Change the palette and the downloads follow.
//
// SVG here; `scripts/render-brand.mjs` renders the PNG of each, because a
// slide deck and a print shop take PNG and this is the set they get.
function tokenColour(css, name, theme) {
  // tokens.css carries the hex in a comment beside each oklch value, which is
  // the only place the literal exists — deriving it from oklch here would be a
  // second colour pipeline.
  const blocks = css.split("prefers-color-scheme: dark");
  const scope = theme === "dark" ? (blocks[1] ?? "") : blocks[0];
  return scope.match(new RegExp(`--${name}:[^;]+;\\s*/\\* (#[0-9A-Fa-f]{6})`))?.[1];
}

const tokensCss = read("apps/web/src/app/tokens.css");
// The dark variant keeps the accent.
//
// Both variants are the mark in colour; what changes between them is the ink
// the letters take, because black letters on a dark ground are not letters.
// The mark and the accent stay `--primary` — the dark theme's value of it,
// which is the lighter one, because a light theme's primary often goes muddy
// on near black.
//
// This is allowed and declared (design-system.md § 3). The product has no dark
// *theme*, which is a decision about the site's surfaces; it was never a
// statement that the mark may not appear on a dark one. A logo that loses its
// accent on half the surfaces it lands on is a logo with two identities.
// Undeclared tokens fall back to neutral greys: never another product's palette.
const THEMES = {
  light: {
    ink: tokenColour(tokensCss, "foreground", "light") ?? "#111111",
    primary: tokenColour(tokensCss, "primary", "light") ?? "#333333",
  },
  dark: {
    ink: tokenColour(tokensCss, "foreground", "dark") ?? "#F5F5F5",
    primary: tokenColour(tokensCss, "primary", "dark") ?? "#CCCCCC",
  },
};

const asset = (name, body) => {
  const dest = `apps/web/public/brand/${name}.svg`;
  mkdirSync(dirname(new URL(`../${dest}`, import.meta.url).pathname), { recursive: true });
  writeFileSync(new URL(`../${dest}`, import.meta.url), body);
  console.log(`wrote ${dest}`);
};

for (const [theme, { ink, primary }] of Object.entries(THEMES)) {
  asset(
    `wordmark-${theme}`,
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${g.wordmarkViewBox}" fill="none">\n` +
      `  <g fill="${ink}">${g.LETTERS.map((d) => `<path d="${d}"/>`).join("")}</g>\n` +
      `  ${accentSvg(g.ACCENT, primary)}\n` +
      `  <path fill-rule="evenodd" d="${g.MARK}" fill="${primary}"/>\n</svg>\n`,
  );
  asset(
    `icon-${theme}`,
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${g.markViewBox}" fill="none">\n` +
      `  <path fill-rule="evenodd" d="${g.MARK}" fill="${primary}"/>\n</svg>\n`,
  );
}

const out = "apps/web/src/generated/logo.ts";
mkdirSync(dirname(new URL(`../${out}`, import.meta.url).pathname), { recursive: true });
writeFileSync(new URL(`../${out}`, import.meta.url), lines.join("\n"));
console.log(`wrote ${out}`);

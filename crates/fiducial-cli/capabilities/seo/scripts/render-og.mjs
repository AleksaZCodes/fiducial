// generated/seo.json + the logo primitives  →  apps/web/public/og/**.png
//
// The social image of every route, rendered from the facts the repository
// already holds: the page's own title and description, the real mark, the real
// colours and the real faces.
//
// ## Why these are built, not committed
//
// An OG image is a *picture of a declaration*. The title in it is the title in
// `generated/seo.ts`; the mark in it is `brand/wordmark.svg`. Committing the
// PNGs would put a copy of both in git — a copy that goes stale silently the
// first time a post is retitled or the flame is redrawn, and nothing renders
// from it so nothing catches it. So they are rebuilt before every web build,
// from the same source, and `public/og/` is gitignored.
//
// ## Why not a runtime image route
//
// Because then every crawler fetch runs code, and a social preview would
// depend on the edge being up and on a font being fetchable at request time.
// These are static files served with the rest of the assets.
//
// ## Fonts are vendored, deliberately
//
// `assets/fonts/*.ttf` are the same families the site loads, kept in the repo
// under their SIL Open Font Licence. Fetching them at build time would put a
// network call — and Google — in the critical path of a deploy.
import { Resvg } from "@resvg/resvg-js";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import satori from "satori";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");

const { routes } = JSON.parse(read("apps/web/src/generated/seo.json"));
const logo = JSON.parse(read("apps/web/src/generated/logo.json"));

const display = readFileSync(at("assets/fonts/ChakraPetch.ttf"));
const body = readFileSync(at("assets/fonts/SourceSerif4.ttf"));

// Brand values, read from the generated module rather than retyped: the same
// rule the rest of the product follows for the domain.
const brand = read("apps/web/src/generated/brand.ts");
const pick = (name) => brand.match(new RegExp(`export const ${name} = "([^"]+)"`))?.[1];
const EMBER = pick("primaryColor") ?? "#b85207";
const domain = pick("domain") ?? "";
const GROUND = "#FDFBF8";
const INK = "#150F0C";
const MUTED = "#6D625B";

/** The wordmark as a data URI, so satori can place it as an image. */
const wordmark = () => {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${logo.wordmarkViewBox}">
    <g fill="${INK}">${logo.LETTERS.map((d) => `<path d="${d}"/>`).join("")}</g>
    <g fill="${EMBER}">${logo.DEVICE.map((p) => `<polygon points="${p}"/>`).join("")}</g>
    <path fill-rule="evenodd" d="${logo.FLAME}" fill="${EMBER}"/>
  </svg>`;
  return `data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}`;
};

const WORDMARK = wordmark();
// An <img> in satori is sized by the caller, not by its own viewBox, and a
// missing or NaN dimension drops it from the layout without a word.
const [, , vbW, vbH] = logo.wordmarkViewBox.split(/\s+/).map(Number);
const WORDMARK_W = 300;
const WORDMARK_H = Math.round((WORDMARK_W * vbH) / vbW);
if (!Number.isFinite(WORDMARK_H)) {
  throw new Error(`render-og: cannot read the wordmark viewBox (${logo.wordmarkViewBox})`);
}

/** Longer text gets a smaller face, so a long title still fits its box. */
const titleSize = (t) => (t.length > 70 ? 54 : t.length > 45 ? 64 : 76);

const clamp = (s, n) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);

function card(route) {
  return {
    type: "div",
    props: {
      style: {
        width: 1200,
        height: 630,
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        background: GROUND,
        padding: "72px 80px",
        // The ember edge, on the two sides the chamfered surfaces are weighted.
        borderBottom: `16px solid ${EMBER}`,
      },
      children: [
        {
          type: "div",
          props: {
            style: { display: "flex", flexDirection: "column", gap: 24 },
            children: [
              {
                type: "div",
                props: {
                  style: {
                    fontFamily: "Display",
                    fontSize: titleSize(route.title),
                    lineHeight: 1.1,
                    color: INK,
                    letterSpacing: "-0.01em",
                  },
                  children: clamp(route.title, 110),
                },
              },
              {
                type: "div",
                props: {
                  style: {
                    fontFamily: "Body",
                    fontSize: 30,
                    lineHeight: 1.45,
                    color: MUTED,
                    maxWidth: 940,
                  },
                  children: clamp(route.description, 180),
                },
              },
            ],
          },
        },
        {
          type: "div",
          props: {
            style: { display: "flex", alignItems: "flex-end", justifyContent: "space-between" },
            children: [
              {
                type: "img",
                props: { src: WORDMARK, width: WORDMARK_W, height: WORDMARK_H },
              },
              {
                type: "div",
                props: {
                  style: { fontFamily: "Body", fontSize: 24, color: MUTED },
                  children: domain,
                },
              },
            ],
          },
        },
      ],
    },
  };
}

let count = 0;
for (const route of routes) {
  const svg = await satori(card(route), {
    width: 1200,
    height: 630,
    fonts: [
      { name: "Display", data: display, weight: 600, style: "normal" },
      { name: "Body", data: body, weight: 400, style: "normal" },
    ],
  });
  const png = new Resvg(svg, { fitTo: { mode: "width", value: 1200 } }).render().asPng();
  const dest = `apps/web/public${route.og}`;
  mkdirSync(dirname(at(dest).pathname), { recursive: true });
  writeFileSync(at(dest), png);
  count++;
}
console.log(`rendered ${count} social image(s) → apps/web/public/og/`);

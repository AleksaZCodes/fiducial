// The brand SVGs → PNGs, staged for the bucket.
//
// `pnpm brand:render`, then `pnpm media:push`.
//
// ## Why PNG at all
//
// SVG is the asset to hand anyone who can take it, and the press page offers
// it first. PNG is what a slide deck, a print shop, a conference badge and
// half the CMSes in the world actually accept, so the set is offered in both
// rather than in the one that is technically better.
//
// ## Why this is a script and not a pipeline
//
// The PNGs are media: they live in the bucket, not in git (`docs/media.md`).
// A pipeline's outputs are artifacts `fid derive --check` compares, and
// comparing a binary it cannot regenerate on a machine without the renderer
// is a check that fails for the wrong reason. The SVGs — which ARE derived,
// and are text — stay in the `logo` pipeline where the check works.
import { Resvg } from "@resvg/resvg-js";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("..", import.meta.url));
const read = (p) => readFileSync(join(repo, p), "utf8");

// Width in pixels per asset. A wordmark is wide and a mark is square, so one
// number for both would make one of them useless.
const ASSETS = [
  { name: "wordmark-light", width: 2400 },
  { name: "wordmark-dark", width: 2400 },
  { name: "icon-light", width: 1024 },
  { name: "icon-dark", width: 1024 },
];

for (const { name, width } of ASSETS) {
  const svg = read(`apps/web/public/brand/${name}.svg`);
  // No background: `Resvg` leaves the canvas transparent unless told
  // otherwise, which is the whole point of the set.
  const png = new Resvg(svg, { fitTo: { mode: "width", value: width } }).render().asPng();
  const dest = join(repo, "media", "press/gallery", `${name}.png`);
  mkdirSync(dirname(dest), { recursive: true });
  writeFileSync(dest, png);
  console.log(`  ✓  press/gallery/${name}.png  (${width}px, transparent)`);
}

console.log(`\n${ASSETS.length} asset(s) staged in media/press/gallery — \`pnpm media:push\` uploads them.`);

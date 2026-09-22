// brand SVG marks → PNG, at derive time, gated by `fid derive --check`.
//
// ## Why this exists at all
//
// `pipelines/brand.toml` has carried this note since it was written: "Not yet
// derived here, deliberately: raster favicons/OG images … Those need a
// rendering step (fonts, rasterization) this pipeline does not carry yet."
// Meanwhile every product that needed an OG image grew a one-off script,
// committed the PNGs it produced, and had no way to tell whether the PNG in
// the repository still matched the SVG it came from. That is precisely the
// state `fid derive --check` exists to make impossible, so the raster step
// becomes a pipeline like any other.
//
// ## Why a rasterizer written here, rather than resvg/sharp/puppeteer
//
// Two hard constraints, and between them they rule out every off-the-shelf
// option:
//
// **It must be deterministic.** `fid derive --check` compares bytes. A
// renderer whose output moves with a library version, a font, or a GPU turns
// the freshness gate into a coin toss, and a gate that fails at random is one
// people learn to re-run rather than read. `resvg` and `sharp` are native
// binaries whose output tracks their own version; a headless browser's output
// tracks the browser.
//
// **It must need nothing installed.** A capability declares the tools it
// needs, and a rasterizer that only works where someone has run `apt install`
// is a pipeline that passes on a laptop and fails in CI.
//
// So this renders the subset a brand mark is actually drawn in — paths,
// groups, flat fills, affine transforms — in plain JavaScript, with no
// dependency at all. **Anything outside that subset is a loud error, never a
// silent approximation.** A mark with a gradient renders wrong rather than
// not at all in a permissive renderer, and wrong is the failure that ships:
// nobody reviews the OG image of a page they did not change.
//
// Text is the sharpest case. Rendering `<text>` needs a font, a font is a
// file whose version nobody pins, and two machines with different versions of
// the same family produce different pixels. **Convert text to paths in the
// source SVG.** The error below says so.
//
// ## The determinism boundary, stated honestly
//
// The geometry is exact: IEEE-754 arithmetic, the same everywhere. The PNG
// container is compressed with Node's `zlib`, whose output *can* change
// between Node major versions. So the encoder never rewrites a file whose
// **pixels** already match: an existing PNG is decoded and compared, and left
// untouched when it is already correct. A Node upgrade therefore cannot
// spuriously fail `--check`, and a changed mark still does.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { deflateSync, inflateSync } from "node:zlib";

const ROOT = resolve(process.cwd());
const fail = (msg) => {
  throw new Error(`rasterize: ${msg}`);
};

// ── The declarations ─────────────────────────────────────────────────────────

// `pipelines/raster.toml`'s `outputs` is the list of rasters this product has.
// It is read here rather than restated in `fiducial.toml`, because `fid derive
// --check` already gates that list — a second copy would be the duplication
// this platform exists to delete, and the copy is the one that goes stale.
//
// `[raster]` in fiducial.toml holds only what a path cannot say: where the
// source SVGs live, and any per-mark framing.
// A reader of its own rather than an import of the `content` capability's:
// `derive-content.mjs` exports `parseToml`, but it is a *script* — importing
// it runs the content derivation as a side effect of reading a config file.
// That is a worse coupling than a small parser.
import { parseToml } from "./toml-lite.mjs";

const pipelinePath = join(ROOT, "pipelines/raster.toml");
if (!existsSync(pipelinePath)) fail("pipelines/raster.toml is missing — it declares the outputs");
const pipeline = parseToml(readFileSync(pipelinePath, "utf8"));
const outputs = pipeline.outputs ?? [];

// No outputs declared is **nothing to do**, not an error. Every product that
// installs `brand` installs this pipeline, and most of them want a favicon
// and no OG image; a capability that made `fid derive` fail until you
// configured a feature you never asked for would be a capability people
// uninstall. The declaration is opt-in, so its absence is a valid state.
if (outputs.length === 0) {
  console.log("rasterize: no outputs declared in pipelines/raster.toml — nothing to render");
  process.exit(0);
}

const config = parseToml(readFileSync(join(ROOT, "fiducial.toml"), "utf8"));
const raster = config.raster ?? {};
const sourceDir =
  raster.source_dir ??
  fail(
    "pipelines/raster.toml declares outputs, but [raster] source_dir is not in\n" +
      "       fiducial.toml — there is no way to know which SVGs they come from."
  );
const marks = raster.marks ?? {};

// ── Output paths are the declaration ─────────────────────────────────────────

/**
 * `.../mark-ink-2048.png` → 2048×2048 of `mark-ink.svg`.
 * `.../mark-ink-1200x630.png` → 1200×630 of the same source.
 *
 * The same convention the `brand` pipeline already uses for its five outputs:
 * the file name says which artifact, the pipeline's `outputs` says where it
 * lands. One list, gated, with nothing to keep in step with it.
 */
function parseOutput(out) {
  const file = out.split("/").pop();
  const m = file.match(/^(.+)-(\d+)(?:x(\d+))?\.png$/);
  if (!m) {
    fail(
      `output "${out}" is not named <mark>-<size>.png or <mark>-<w>x<h>.png.\n` +
        `       The name is the declaration: it says which SVG and at what size.`
    );
  }
  const [, name, w, h] = m;
  // `key` is the output's own basename (`mark-ink-1200x630`), which is what a
  // per-output `[raster.marks."…"]` block is named after.
  return { out, name, key: file.replace(/\.png$/, ""), width: Number(w), height: Number(h ?? w) };
}

// ── SVG, the subset a mark is drawn in ───────────────────────────────────────

function parseSvg(src, where) {
  const root = src.match(/<svg\b([^>]*)>/i);
  if (!root) fail(`${where} has no <svg> element`);
  const viewBox = attr(root[1], "viewBox");
  if (!viewBox) fail(`${where} has no viewBox — there is no way to know its coordinate space`);
  const [vx, vy, vw, vh] = viewBox.trim().split(/[\s,]+/).map(Number);

  for (const el of ["text", "image", "use", "linearGradient", "radialGradient", "filter", "mask", "clipPath"]) {
    if (new RegExp(`<${el}\\b`, "i").test(src)) {
      fail(
        `${where} contains <${el}>, which this renderer will not approximate.\n` +
          (el === "text"
            ? "       Text needs a font, and two machines with different versions of the\n" +
              "       same family produce different pixels — so the artifact would not be\n" +
              "       reproducible. Convert the text to paths in the source SVG."
            : `       Supported: <g>, <path>, flat fills, and affine transforms.`)
      );
    }
  }

  // A flat walk, not a tree: `<g>` may nest, so the fill and transform in
  // scope are tracked on a stack as the tags go by. Enough for a mark, and it
  // fails loudly on anything it is not enough for.
  const shapes = [];
  const stack = [{ fill: "#000000", matrix: [1, 0, 0, 1, 0, 0] }];
  const tag = /<(\/?)(g|path|svg)\b([^>]*?)(\/?)>/gi;
  let m;
  while ((m = tag.exec(src)) !== null) {
    const [, closing, name, attrs, selfClosing] = m;
    const lower = name.toLowerCase();
    if (lower === "svg") continue;
    if (closing) {
      if (lower === "g") stack.pop();
      continue;
    }
    const top = stack[stack.length - 1];
    const fill = attr(attrs, "fill") ?? top.fill;
    const matrix = multiply(top.matrix, parseTransform(attr(attrs, "transform")));
    if (lower === "g") {
      if (!selfClosing) stack.push({ fill, matrix });
      continue;
    }
    const d = attr(attrs, "d");
    if (!d) fail(`${where} has a <path> with no d attribute`);
    if (fill === "none") continue;
    shapes.push({ subpaths: flatten(d, matrix, where), color: parseColor(fill, where) });
  }
  if (shapes.length === 0) fail(`${where} has no filled paths`);
  return { vx, vy, vw, vh, shapes };
}

function attr(attrs, name) {
  const m = attrs.match(new RegExp(`\\b${name}\\s*=\\s*"([^"]*)"`, "i"));
  return m ? m[1] : undefined;
}

function parseTransform(value) {
  if (!value) return [1, 0, 0, 1, 0, 0];
  let out = [1, 0, 0, 1, 0, 0];
  const fn = /([a-zA-Z]+)\s*\(([^)]*)\)/g;
  let m;
  while ((m = fn.exec(value)) !== null) {
    const n = m[2].trim().split(/[\s,]+/).map(Number);
    let t;
    switch (m[1]) {
      case "translate":
        t = [1, 0, 0, 1, n[0], n[1] ?? 0];
        break;
      case "scale":
        t = [n[0], 0, 0, n[1] ?? n[0], 0, 0];
        break;
      case "matrix":
        t = n.slice(0, 6);
        break;
      case "rotate": {
        const r = (n[0] * Math.PI) / 180;
        const [cos, sin] = [Math.cos(r), Math.sin(r)];
        t = [cos, sin, -sin, cos, 0, 0];
        if (n.length === 3) {
          t = multiply(multiply([1, 0, 0, 1, n[1], n[2]], t), [1, 0, 0, 1, -n[1], -n[2]]);
        }
        break;
      }
      default:
        fail(`unsupported transform "${m[1]}" — supported: translate, scale, rotate, matrix`);
    }
    out = multiply(out, t);
  }
  return out;
}

const multiply = (a, b) => [
  a[0] * b[0] + a[2] * b[1],
  a[1] * b[0] + a[3] * b[1],
  a[0] * b[2] + a[2] * b[3],
  a[1] * b[2] + a[3] * b[3],
  a[0] * b[4] + a[2] * b[5] + a[4],
  a[1] * b[4] + a[3] * b[5] + a[5],
];
const apply = (m, x, y) => [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];

function parseColor(value, where) {
  const v = value.trim();
  const hex = v.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (hex) {
    const h = hex[1];
    const full = h.length === 3 ? [...h].map((c) => c + c).join("") : h;
    return [
      Number.parseInt(full.slice(0, 2), 16),
      Number.parseInt(full.slice(2, 4), 16),
      Number.parseInt(full.slice(4, 6), 16),
    ];
  }
  fail(
    `${where} uses fill "${value}". Only #rgb and #rrggbb are accepted.\n` +
      `       A named colour or a gradient is a colour this renderer would have to\n` +
      `       guess at, and a guess that renders is worse than an error that does not.`
  );
}

// ── Paths → polygons ─────────────────────────────────────────────────────────

/** How finely a curve is chopped. Fixed, so the output cannot drift. */
const CURVE_STEPS = 24;

function flatten(d, matrix, where) {
  const tokens = d.match(/[a-zA-Z]|-?\d*\.?\d+(?:e[-+]?\d+)?/gi) ?? [];
  const subpaths = [];
  let current = null;
  let [x, y] = [0, 0];
  let [startX, startY] = [0, 0];
  let [lastCx, lastCy] = [0, 0];
  let cmd = null;
  let i = 0;

  const num = () => {
    const v = Number(tokens[i++]);
    if (Number.isNaN(v)) fail(`${where} has a malformed path near token ${i}`);
    return v;
  };
  const move = (nx, ny) => {
    x = nx;
    y = ny;
    current.push(apply(matrix, x, y));
  };
  const curve = (x1, y1, x2, y2, x3, y3) => {
    const [x0, y0] = [x, y];
    for (let s = 1; s <= CURVE_STEPS; s++) {
      const t = s / CURVE_STEPS;
      const u = 1 - t;
      const px = u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3;
      const py = u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3;
      current.push(apply(matrix, px, py));
    }
    [lastCx, lastCy] = [x2, y2];
    [x, y] = [x3, y3];
  };

  while (i < tokens.length) {
    if (/^[a-zA-Z]$/.test(tokens[i])) cmd = tokens[i++];
    else if (cmd === "M") cmd = "L";
    else if (cmd === "m") cmd = "l";
    const rel = cmd === cmd.toLowerCase();
    const [ox, oy] = rel ? [x, y] : [0, 0];

    switch (cmd.toUpperCase()) {
      case "M":
        if (current?.length) subpaths.push(current);
        current = [];
        [startX, startY] = [ox + num(), oy + num()];
        move(startX, startY);
        break;
      case "L":
        move(ox + num(), oy + num());
        break;
      case "H":
        move(ox + num(), y);
        break;
      case "V":
        move(x, oy + num());
        break;
      case "C":
        curve(ox + num(), oy + num(), ox + num(), oy + num(), ox + num(), oy + num());
        break;
      case "S": {
        // The reflection of the previous control point — that is what makes S
        // smooth. With no previous curve, the reflection is the point itself.
        const [rx, ry] = "CS".includes((tokens[i - 2] ?? "").toUpperCase())
          ? [2 * x - lastCx, 2 * y - lastCy]
          : [x, y];
        curve(rx, ry, ox + num(), oy + num(), ox + num(), oy + num());
        break;
      }
      case "Q": {
        const [qx, qy] = [ox + num(), oy + num()];
        const [ex, ey] = [ox + num(), oy + num()];
        // A quadratic is a cubic with both control points a third of the way
        // in — exactly, not approximately.
        curve(x + (2 / 3) * (qx - x), y + (2 / 3) * (qy - y), ex + (2 / 3) * (qx - ex), ey + (2 / 3) * (qy - ey), ex, ey);
        break;
      }
      case "Z":
        if (current?.length) {
          current.push(apply(matrix, startX, startY));
          subpaths.push(current);
          current = [];
        }
        [x, y] = [startX, startY];
        break;
      case "A":
        fail(
          `${where} uses an arc (A) command, which this renderer does not implement.\n` +
            `       Most vector editors can export arcs as cubic curves.`
        );
        break;
      case "T":
        fail(`${where} uses T, which this renderer does not implement — use Q.`);
        break;
      default:
        fail(`${where} uses unsupported path command "${cmd}"`);
    }
  }
  if (current?.length) subpaths.push(current);
  return subpaths.filter((p) => p.length > 2);
}

// ── Scanline fill ────────────────────────────────────────────────────────────

/** Samples per pixel per axis. Fixed, so antialiasing cannot drift. */
const SS = 4;

/**
 * Coverage per pixel for one shape, by the nonzero winding rule.
 *
 * Nonzero, not even-odd, and it matters here: the counter of a `B` is a
 * subpath wound the other way, and even-odd would fill it solid on a mark
 * whose subpaths happen to share a winding direction.
 */
function coverage(subpaths, width, height) {
  const cov = new Float32Array(width * height);
  const edges = [];
  let minY = Infinity;
  let maxY = -Infinity;
  for (const pts of subpaths) {
    for (let k = 0; k < pts.length - 1; k++) {
      const [x0, y0] = pts[k];
      const [x1, y1] = pts[k + 1];
      if (y0 === y1) continue;
      edges.push([x0, y0, x1, y1]);
      minY = Math.min(minY, y0, y1);
      maxY = Math.max(maxY, y0, y1);
    }
    // An unclosed subpath still bounds an area when filled — SVG closes it
    // implicitly, so the renderer must too.
    const [fx, fy] = pts[0];
    const [lx, ly] = pts[pts.length - 1];
    if (fx !== lx || fy !== ly) {
      edges.push([lx, ly, fx, fy]);
      minY = Math.min(minY, ly, fy);
      maxY = Math.max(maxY, ly, fy);
    }
  }
  if (edges.length === 0) return cov;

  const yStart = Math.max(0, Math.floor(minY));
  const yEnd = Math.min(height - 1, Math.ceil(maxY));
  const share = 1 / (SS * SS);
  const crossings = [];

  for (let py = yStart; py <= yEnd; py++) {
    for (let sy = 0; sy < SS; sy++) {
      const y = py + (sy + 0.5) / SS;
      crossings.length = 0;
      for (const [x0, y0, x1, y1] of edges) {
        if (y < Math.min(y0, y1) || y >= Math.max(y0, y1)) continue;
        const t = (y - y0) / (y1 - y0);
        crossings.push([x0 + t * (x1 - x0), y1 > y0 ? 1 : -1]);
      }
      if (crossings.length === 0) continue;
      crossings.sort((a, b) => a[0] - b[0]);

      let winding = 0;
      for (let c = 0; c < crossings.length - 1; c++) {
        winding += crossings[c][1];
        if (winding === 0) continue;
        // A span of sub-samples is filled between two crossings; walking
        // sub-samples instead of whole pixels is what produces the soft edge.
        const spanStart = crossings[c][0];
        const spanEnd = crossings[c + 1][0];
        const first = Math.max(0, Math.ceil(spanStart * SS - 0.5));
        const last = Math.min(width * SS - 1, Math.floor(spanEnd * SS - 0.5));
        for (let sx = first; sx <= last; sx++) {
          cov[py * width + Math.floor(sx / SS)] += share;
        }
      }
    }
  }
  return cov;
}

// ── PNG ──────────────────────────────────────────────────────────────────────

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const PNG_MAGIC = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

function encodePng(rgba, width, height) {
  // Filter 0 (None) on every row. A per-row adaptive filter would compress
  // better and would make the bytes depend on a heuristic — the smaller file
  // is not worth a byte-level output that is harder to reason about.
  const raw = Buffer.alloc(height * (1 + width * 4));
  for (let y = 0; y < height; y++) {
    raw[y * (1 + width * 4)] = 0;
    rgba.copy(raw, y * (1 + width * 4) + 1, y * width * 4, (y + 1) * width * 4);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type: RGBA
  return Buffer.concat([
    PNG_MAGIC,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * The pixels of an existing PNG this script wrote, or null.
 *
 * This is the whole determinism story: the encoder's *bytes* depend on the
 * zlib that Node happens to ship, but the *pixels* do not depend on anything.
 * Comparing pixels and skipping the write means a Node upgrade cannot fail
 * `fid derive --check` on a mark nobody touched.
 */
function decodePngPixels(buf) {
  if (!buf.subarray(0, 8).equals(PNG_MAGIC)) return null;
  let off = 8;
  let width = 0;
  let height = 0;
  const idat = [];
  while (off + 8 <= buf.length) {
    const len = buf.readUInt32BE(off);
    const type = buf.toString("ascii", off + 4, off + 8);
    const data = buf.subarray(off + 8, off + 8 + len);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      if (data[8] !== 8 || data[9] !== 6) return null;
    } else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    off += 12 + len;
  }
  if (!width || idat.length === 0) return null;
  let raw;
  try {
    raw = inflateSync(Buffer.concat(idat));
  } catch {
    return null;
  }
  const stride = 1 + width * 4;
  if (raw.length !== height * stride) return null;
  const out = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    if (raw[y * stride] !== 0) return null; // not ours; re-render
    raw.copy(out, y * width * 4, y * stride + 1, (y + 1) * stride);
  }
  return { width, height, pixels: out };
}

// ── Render ───────────────────────────────────────────────────────────────────

function render(svg, spec, options) {
  const { width, height } = spec;
  const rgba = Buffer.alloc(width * height * 4);

  const bg = options.background ? parseColor(options.background, spec.out) : null;
  if (bg) {
    for (let p = 0; p < width * height; p++) {
      rgba[p * 4] = bg[0];
      rgba[p * 4 + 1] = bg[1];
      rgba[p * 4 + 2] = bg[2];
      rgba[p * 4 + 3] = 255;
    }
  }

  // The box that gets centred. `artbox` exists because centring the full
  // drawing is usually wrong for a logo: a mark with an ascender rising out of
  // the letterforms sits visibly low when the ascender is counted as part of
  // what to centre. Declaring the cap box instead centres what a reader reads
  // as the mark, and lets the rest hang outside it.
  const box = options.artbox
    ? String(options.artbox).trim().split(/[\s,]+/).map(Number)
    : [svg.vx, svg.vy, svg.vw, svg.vh];
  if (box.length !== 4 || box.some(Number.isNaN)) {
    fail(`[raster.marks.${spec.name}] artbox must be four numbers: "x y width height"`);
  }
  const [bx, by, bw, bh] = box;

  // `padding` is a fraction of the short edge, so one number reads the same on
  // a square avatar and a wide banner.
  const pad = Number(options.padding ?? 0);
  if (!(pad >= 0 && pad < 0.5)) fail(`[raster.marks.${spec.name}] padding must be in [0, 0.5)`);
  const inset = Math.min(width, height) * pad;
  const availW = width - 2 * inset;
  const availH = height - 2 * inset;
  const scale = Math.min(availW / bw, availH / bh);
  const dx = (width - bw * scale) / 2 - bx * scale;
  const dy = (height - bh * scale) / 2 - by * scale;
  const place = [scale, 0, 0, scale, dx, dy];

  for (const shape of svg.shapes) {
    const placed = shape.subpaths.map((pts) => pts.map(([x, y]) => apply(place, x, y)));
    const cov = coverage(placed, width, height);
    const [r, g, b] = shape.color;
    for (let p = 0; p < cov.length; p++) {
      const a = Math.min(1, cov[p]);
      if (a <= 0) continue;
      const o = p * 4;
      // Source-over, on straight (non-premultiplied) RGBA.
      const dstA = rgba[o + 3] / 255;
      const outA = a + dstA * (1 - a);
      rgba[o] = Math.round((r * a + rgba[o] * dstA * (1 - a)) / outA);
      rgba[o + 1] = Math.round((g * a + rgba[o + 1] * dstA * (1 - a)) / outA);
      rgba[o + 2] = Math.round((b * a + rgba[o + 2] * dstA * (1 - a)) / outA);
      rgba[o + 3] = Math.round(outA * 255);
    }
  }
  return rgba;
}

// ── Main ─────────────────────────────────────────────────────────────────────

let written = 0;
let unchanged = 0;

for (const out of outputs) {
  const spec = parseOutput(out);
  const svgPath = join(ROOT, sourceDir, `${spec.name}.svg`);
  if (!existsSync(svgPath)) {
    fail(
      `output "${out}" needs ${sourceDir}/${spec.name}.svg, which does not exist.\n` +
        `       The output's name is what names its source.`
    );
  }
  const svg = parseSvg(readFileSync(svgPath, "utf8"), `${sourceDir}/${spec.name}.svg`);

  // Options resolve per mark, then per output, with the more specific winning
  // key by key. Framing is not a property of a mark alone: the same logo
  // wants its cap box centred on a square avatar and its whole drawing
  // centred on a 1200×630 banner, because an ascender that hangs outside the
  // box on a square is simply cropped off a wide one. Without this, the
  // second size silently loses part of the mark — which is exactly what the
  // first run of this pipeline did.
  const options = { ...(marks[spec.name] ?? {}), ...(marks[spec.key] ?? {}) };
  const pixels = render(svg, spec, options);

  const abs = join(ROOT, out);
  if (existsSync(abs)) {
    const existing = decodePngPixels(readFileSync(abs));
    if (
      existing &&
      existing.width === spec.width &&
      existing.height === spec.height &&
      existing.pixels.equals(pixels)
    ) {
      unchanged++;
      continue;
    }
  }
  mkdirSync(dirname(abs), { recursive: true });
  writeFileSync(abs, encodePng(pixels, spec.width, spec.height));
  written++;
}

console.log(
  `rasterize: ${written} written, ${unchanged} already correct (${outputs.length} declared)`
);

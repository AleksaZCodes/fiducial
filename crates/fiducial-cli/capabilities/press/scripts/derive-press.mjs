// press/ + [press] + [brand]  →  one typed module the press page renders.
//
// The press room is a derivation, not a page someone maintains. A press kit
// that is hand-written drifts from the brand it describes: the boilerplate
// keeps a company name that changed, the fast facts keep a headcount from two
// years ago, and the logo section links a file that was redrawn. Every one of
// those is a wrong fact in circulation, because a press kit's whole purpose is
// to be copied.
//
// So the facts come from the declarations that already own them — the company
// name and domain from `[brand]`, the logo from the two brand primitives — and
// only the prose a person must write lives in `press/`.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { frontmatter as fm } from "./frontmatter.mjs";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");

/**
 * Minimal TOML reach-in: the block and keys we need, nothing more.
 *
 * Parsed line by line rather than with one big regex. The regex version had two
 * bugs and both were silent:
 *
 *   · It ended the block with a `(?=^\[|\Z)` lookahead. **`\Z` is not a JS
 *     escape** — it is a literal `Z` — so the block stopped at the first
 *     capital Z in any value. It was found because a spokesperson named
 *     Zdravković came out of the pipeline as `"Aleksa`.
 *   · It stripped `#` to the end of line from every value, to drop trailing
 *     comments. That corrupts any quoted value containing a `#`, which is
 *     every hex colour — and `[brand]` is full of them.
 *
 * A quoted value is taken verbatim. Only an unquoted one can carry a trailing
 * comment, so only an unquoted one is stripped.
 */
function tomlBlock(src, name) {
  const out = {};
  let inside = false;
  let pending = null; // key whose array value spans lines

  for (const raw of src.split("\n")) {
    const line = raw.trim();

    // A multi-line array. TOML allows `locales = [` … `]` across lines and
    // `[i18n]` is normally written that way, so a line-wise parser that stops
    // at the newline reads the value as `[` and reports no locales at all.
    if (pending !== null) {
      pending.buf += ` ${line}`;
      if (line.includes("]")) {
        out[pending.key] = pending.buf.trim();
        pending = null;
      }
      continue;
    }

    if (/^\[[^\]]+\]$/.test(line)) {
      inside = line === `[${name}]`;
      continue;
    }
    if (!inside || !line || line.startsWith("#")) continue;

    const kv = raw.match(/^\s*([A-Za-z_][\w-]*)\s*=\s*(.*)$/);
    if (!kv) continue;
    let v = kv[2].trim();

    if (v.startsWith("[") && !v.includes("]")) {
      pending = { key: kv[1], buf: v };
      continue;
    }
    if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1);
    else if (!v.startsWith("[")) v = v.replace(/\s*#.*$/, "").trim();
    out[kv[1]] = v;
  }
  return out;
}

/** A TOML array literal (single or multi-line) → string[]. */
function tomlArray(v) {
  return String(v ?? "")
    .replace(/^\[|\]$/g, "")
    .split(",")
    .map((s) => s.trim().replace(/^"|"$/g, ""))
    .filter(Boolean);
}

const cfg = read("fiducial.toml");
const press = tomlBlock(cfg, "press");
const brand = tomlBlock(cfg, "brand");

/** Strip HTML comments — they are guidance for the writer, not content. */
const decomment = (s) => s.replace(/<!--[\s\S]*?-->/g, "").trim();

/** `## name` sections → { name: body }. */
function sections(md) {
  const out = {};
  const parts = decomment(md)
    .split(/^##\s+/m)
    .slice(1);
  for (const p of parts) {
    const nl = p.indexOf("\n");
    out[p.slice(0, nl).trim()] = p.slice(nl + 1).trim();
  }
  return out;
}

/** `key: value` lines → ordered pairs. Order is editorial, so it is kept. */
function pairs(md) {
  return decomment(md)
    .split("\n")
    .map((l) => l.match(/^([^:]+):\s*(.+)$/))
    .filter(Boolean)
    .map((m) => ({ label: m[1].trim(), value: m[2].trim() }));
}

/** `---` frontmatter + body. */
function frontmatter(md) {
  // A story with no frontmatter is a boilerplate or facts document, which is
  // body only — not an error.
  if (!/^---\r?\n/.test(md)) return { meta: {}, body: decomment(md) };
  const { meta, body } = fm(md, (msg) => {
    throw new Error(`derive-press: ${msg}`);
  });
  return { meta, body: decomment(body) };
}


/**
 * The locales, read from `[i18n]`.
 *
 * This capability requires `i18n`, so the block is always there. Copy is a
 * fact and a fact has one derivation per locale.
 */
const i18n = tomlBlock(cfg, "i18n");
const locales = tomlArray(i18n.locales);
if (locales.length === 0) {
  throw new Error("derive-press: [i18n] declares no locales");
}

/** Load one locale's press room, or say exactly what is missing. */
function loadLocale(loc) {
  // The locale sits UNDER each collection — press/docs/<locale>/… and
  // press/stories/<locale>/… — the same nesting `content/` uses. It was the
  // other way round, which reads fine on disk and is the one arrangement an
  // editor cannot show two languages of side by side.
  const base = `press/docs/${loc}`;
  const need = [`${base}/boilerplate.md`, `${base}/facts.md`];
  const absent = need.filter((f) => !existsSync(at(f)));
  if (absent.length) {
    // MISSION.md 1c: a missing translation is a MISSING ARTIFACT, not a
    // fallback. Falling back to the default locale here is the tempting
    // behaviour and the wrong one — it ships a press kit that silently serves
    // English boilerplate to a Serbian journalist, who quotes it, and nobody
    // finds out.
    throw new Error(
      `derive-press: locale \`${loc}\` is declared in [i18n] but its press room ` +
        `is incomplete.\n\n  missing:\n` +
        absent.map((f) => `    ${f}`).join("\n") +
        `\n\n  A missing translation is a missing artifact, not a fallback ` +
        `(MISSION.md 1c).\n  Write them, or remove \`${loc}\` from [i18n] locales.`,
    );
  }
  const storyDir = at(`press/stories/${loc}`);
  const stories = (existsSync(storyDir) ? readdirSync(storyDir) : [])
    .filter((f) => f.endsWith(".md"))
    .sort()
    .map((f) => {
      const { meta, body } = frontmatter(read(`press/stories/${loc}/${f}`));
      return {
        slug: f.replace(/\.md$/, ""),
        title: meta.title ?? f,
        angle: meta.angle ?? "",
        date: meta.date ?? "",
        summary: meta.summary ?? "",
        // A media key, not a URL: the renderer resolves it through the storage
        // adapter, so a story never knows which vendor holds its picture.
        cover: meta.cover ?? "",
        coverAlt: meta.cover_alt ?? "",
        body,
      };
    });
  return {
    // The BODY, not the file. These documents carry frontmatter now — a title
    // the editor lists them by — and reading the raw file turned that line
    // into a fact: the press room published "title: Brze činjenice" as the
    // first row of its fast-facts table.
    boilerplate: sections(frontmatter(read(`${base}/boilerplate.md`)).body),
    facts: pairs(frontmatter(read(`${base}/facts.md`)).body),
    stories,
  };
}

// A press room with no contact is worse than no press room: a journalist who
// cannot reach you writes the piece anyway, without the quote checked.
//
// This exists because the block went missing and nothing noticed. `[press]` was
// dropped from fiducial.toml by an unrelated edit, every field read as an empty
// string, the page rendered with a blank contact section, and the build stayed
// green. An empty required fact fails here, where it is cheap, rather than on a
// page somebody is reading on deadline.
if (!press.press_email) {
  throw new Error(
    "derive-press: [press] press_email is empty or missing from fiducial.toml.\n\n" +
      "  A press kit exists to be used. Without a contact address it is a page that\n" +
      "  tells a journalist to go and find someone else.",
  );
}

const byLocale = Object.fromEntries(locales.map((l) => [l, loadLocale(l)]));

// Every locale must carry the same stories. A press room that has the failure
// story in one language and not the other is not translated, it is two
// different press rooms — and the one missing it is the one that reads as
// marketing.
const slugSets = locales.map((l) =>
  byLocale[l].stories
    .map((s) => s.slug)
    .sort()
    .join("|"),
);
if (new Set(slugSets).size > 1) {
  const detail = locales
    .map((l) => `    ${l}: ${byLocale[l].stories.map((s) => s.slug).join(", ") || "(none)"}`)
    .join("\n");
  throw new Error(
    `derive-press: the locales do not carry the same stories.\n\n${detail}\n\n` +
      `  Every story exists in every locale, or it exists in none.`,
  );
}

const out = {
  locales,
  pressEmail: press.press_email ?? "",
  founded: press.founded ?? "",
  hq: press.hq ?? "",
  spokesperson: press.spokesperson ?? "",
  legalName: brand.legal_name ?? "",
  tradingName: brand.trading_name ?? "",
  domain: brand.domain ?? "",
  byLocale,
};

const lines = [
  "// Generated by `fid derive` (pipeline: press) from press/ and fiducial.toml.",
  "// Do not edit — edit the declarations and re-derive.",
  "//",
  "// A press kit exists to be copied, so a stale fact here becomes a wrong fact",
  "// in someone else's article, where you cannot correct it. Everything a",
  "// declaration already owns is read from that declaration, never retyped.",
  "",
  `export const press = ${JSON.stringify(out, null, 2)} as const`,
  "",
  "export type PressLocale = (typeof press.locales)[number]",
  'export type Story = (typeof press.byLocale)[PressLocale]["stories"][number]',
  "",
  "/** One locale's press room. A locale absent here failed the build, not this call. */",
  "export const pressFor = (locale: PressLocale) => press.byLocale[locale]",
  "",
];
const target = "apps/web/src/generated/press.ts";
mkdirSync(dirname(at(target).pathname), { recursive: true });
writeFileSync(at(target), lines.join("\n"));
console.log(
  `wrote ${target} (${locales.length} locale(s), ${byLocale[locales[0]].stories.length} story/stories each)`,
);

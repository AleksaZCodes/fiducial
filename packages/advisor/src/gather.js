/**
 * The deterministic half of every check.
 *
 * Each advisory question is a hybrid: code does a cheap, over-inclusive,
 * perfectly repeatable pass, and the model judges only what survives. This
 * module is that first pass. Nothing here asks anything — it reads git and the
 * working tree and returns plain data.
 *
 * Keeping the split this sharp buys two things. The model's input becomes small
 * and focused, which is most of what makes an answer good and all of what makes
 * it cheap. And every candidate a finding refers to is one code found, so a
 * report can always point at a real line instead of describing something the
 * model believes it saw.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

/** Run git, returning stdout, or `null` when git fails or is absent. */
function git(args, cwd) {
  try {
    return execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch {
    return null;
  }
}

/**
 * The diff to review, and the files it touches.
 *
 * Defaults to everything not yet committed — staged, unstaged, and untracked —
 * because that is what an agent just produced and what a person is about to
 * commit. A `base` ref switches to reviewing a whole branch instead.
 *
 * Untracked files are read from disk and rendered as synthetic additions rather
 * than made visible to `git diff` with `--intent-to-add`; see the comment at
 * that step for why the index must not be touched.
 */
export function collectDiff(cwd, base) {
  if (base) {
    const merged = git(["diff", "--no-color", `${base}...HEAD`], cwd);
    const names = git(["diff", "--name-only", `${base}...HEAD`], cwd);
    return {
      diff: merged ?? "",
      files: (names ?? "").split("\n").filter(Boolean),
      scope: `commits since ${base}`,
    };
  }

  const tracked = git(["diff", "--no-color", "HEAD"], cwd) ?? git(["diff", "--no-color"], cwd) ?? "";
  const names = git(["diff", "--name-only", "HEAD"], cwd) ?? git(["diff", "--name-only"], cwd) ?? "";
  const files = names.split("\n").filter(Boolean);

  // Untracked files are read directly and rendered as synthetic additions.
  //
  // The tempting shortcut is `git add -N` first, which makes them visible to
  // `git diff` in one command — and it is wrong here. Intent-to-add **writes to
  // the index**, so it changes what `git status` reports and what a subsequent
  // bare `git commit` would include. This command promises to change nothing,
  // and a tool that quietly stages files on the way to giving advice has broken
  // that promise in the most confusing possible place.
  //
  // A new file is also the likeliest home for a fresh violation, so skipping
  // them is not an option either.
  const untracked = (git(["ls-files", "--others", "--exclude-standard"], cwd) ?? "")
    .split("\n")
    .filter(Boolean);

  let synthetic = "";
  for (const rel of untracked.slice(0, 40)) {
    const body = readIfText(cwd, rel);
    if (body === null) continue;
    files.push(rel);
    const lines = body.split("\n");
    synthetic +=
      `diff --git a/${rel} b/${rel}\n` +
      `new file mode 100644\n` +
      `--- /dev/null\n` +
      `+++ b/${rel}\n` +
      `@@ -0,0 +1,${lines.length} @@\n` +
      lines.map((l) => `+${l}`).join("\n") +
      "\n";
  }

  return {
    diff: tracked + synthetic,
    files,
    scope: "uncommitted changes",
  };
}

/**
 * Read a file only if it is plausibly reviewable text.
 *
 * Binary content and very large files are skipped rather than truncated: a
 * model shown the first 60KB of a PNG will still answer the questions, and the
 * answer will be about nothing.
 */
function readIfText(cwd, rel, maxBytes = 200_000) {
  try {
    const abs = join(cwd, rel);
    const stat = statSync(abs);
    if (!stat.isFile() || stat.size > maxBytes) return null;
    const buf = readFileSync(abs);
    // A NUL byte in the first block is the same heuristic git itself uses.
    if (buf.subarray(0, 8000).includes(0)) return null;
    return buf.toString("utf8");
  } catch {
    return null;
  }
}

/** Added lines of a unified diff, with their file and line number. */
export function addedLines(diff) {
  const out = [];
  let file = null;
  let lineNo = 0;

  for (const raw of diff.split("\n")) {
    if (raw.startsWith("+++ b/")) {
      file = raw.slice("+++ b/".length);
      continue;
    }
    const hunk = /^@@ -\d+(?:,\d+)? \+(\d+)/.exec(raw);
    if (hunk) {
      lineNo = Number(hunk[1]);
      continue;
    }
    if (raw.startsWith("+") && !raw.startsWith("+++")) {
      out.push({ file, line: lineNo, text: raw.slice(1) });
      lineNo++;
      continue;
    }
    if (!raw.startsWith("-") && !raw.startsWith("\\")) lineNo++;
  }
  return out;
}

/**
 * Numeric literals bound to a name, as candidates for "should this be a fact?".
 *
 * Over-inclusive by design. Telling a board thickness from a retry limit is a
 * judgment and belongs to the model; finding `name = 1.6` is a regex and
 * belongs here. The exclusions below are only the cases that are *certainly*
 * not physical quantities — the ones where a rule is exact, so no call is spent
 * on them:
 *
 *   - `0` and `1`, which are almost always sentinels, flags or indices
 *   - values inside an obvious version or status context
 *   - lines that already name a unit, since those are already doing it right
 *
 * Anything ambiguous is passed along. A false candidate costs a fraction of a
 * cent; a missed bare quantity is the violation this platform exists to stop.
 */
export function numericCandidates(added, limit = 40) {
  const out = [];
  const seen = new Set();

  // A named binding to a number: `const X = 1.6`, `x: 1.6`, `X = 1.6`.
  const pattern = /([A-Za-z_][A-Za-z0-9_]{2,})\s*[:=]\s*(-?\d+(?:\.\d+)?)/g;

  for (const { file, line, text } of added) {
    // Test files are excluded wholesale. A fixture value is not a declaration —
    // it is an assertion about one — so a test asserting `noul = 0.96` is not a
    // physical quantity missing its unit. Left in, fixtures dominated the
    // candidate list and spent the model's attention on guaranteed noise.
    if (file && /(\.|_)(test|spec)\.|(^|\/)(tests?|__tests__|fixtures?)\//.test(file)) continue;
    // Prose is not where quantities are declared. A markdown file's frontmatter
    // holds dates and slugs, and scanning it produced `date = 2026` as a
    // candidate physical quantity — a false positive that costs a call and
    // teaches the reader to distrust the check.
    if (file && /\.(md|mdx|markdown|txt|rst|adoc)$/i.test(file)) continue;
    // Already declared with a unit, or a comment — nothing to judge.
    if (/\b(mm|cm|um|µm|nm|mil|inch|kg|g|mg|mA|uA|A|mV|V|kV|Hz|kHz|MHz|GHz|ms|us|ns|s|°C|K|N|Pa|kPa|ohm|Ω)\b/.test(text)) {
      continue;
    }
    const code = text.trim();
    if (code.startsWith("//") || code.startsWith("#") || code.startsWith("*")) continue;

    for (const m of code.matchAll(pattern)) {
      const [, name, value] = m;
      const n = Number(value);
      if (n === 0 || n === 1 || n === -1) continue;
      if (/version|status|code|port|index|idx|count|len|length|size|max_?retries|timeout_?ms|attempts|limit|offset|width_?px|height_?px|z_?index|opacity|flex|span|cols?|rows?|tokens?|bytes?|chars?|date|year|month|day|hour|minute|seq|order|priority|weight_?class/i.test(name)) {
        continue;
      }
      // A unit-bearing probability is a contradiction, so a name from this
      // family plus a value in [0,1] is certainly dimensionless. Exact rule,
      // so no call is spent on it.
      if (
        n > 0 &&
        n < 1 &&
        /prob|confidence|noul|score|weight|threshold|ratio|alpha|fraction|percent|rate|delay|jitter/i.test(
          name,
        )
      ) {
        continue;
      }
      const fingerprint = `${name}=${value}`;
      if (seen.has(fingerprint)) continue;
      seen.add(fingerprint);
      out.push({ file, line, name, value: n, source: code.slice(0, 160) });
      if (out.length >= limit) return out;
    }
  }
  return out;
}

/**
 * Declaration-shaped names already present in the repository.
 *
 * Context for the duplicate-declaration question: without it the model can only
 * spot a value repeated twice *inside* the diff, which is the easy half. The
 * interesting violation is a diff that hardcodes something already declared
 * somewhere it never looked.
 *
 * Read from the declaration files this platform actually uses — `fiducial.toml`
 * and `pipelines/*.toml` — rather than by scanning all source, because a
 * declaration is supposed to live in one of those, and scanning everything
 * would bury the signal in ordinary code.
 */
export function existingDeclarations(cwd, limit = 200) {
  const out = {};
  const files = (git(["ls-files", "fiducial.toml", "*.toml", "pipelines/*.toml"], cwd) ?? "")
    .split("\n")
    .filter(Boolean)
    .slice(0, 40);

  for (const rel of files) {
    let text;
    try {
      text = execFileSync("cat", [rel], { cwd, encoding: "utf8" });
    } catch {
      continue;
    }
    let section = "";
    for (const raw of text.split("\n")) {
      const line = raw.trim();
      if (line.startsWith("#") || !line) continue;
      const sec = /^\[+([^\]]+)\]+$/.exec(line);
      if (sec) {
        section = sec[1];
        continue;
      }
      const eq = line.indexOf("=");
      if (eq === -1) continue;
      const name = line.slice(0, eq).trim();
      const value = line.slice(eq + 1).trim().slice(0, 80);
      if (!name || name.includes(" ")) continue;
      const key = section ? `${section}.${name}` : name;
      if (!(key in out)) out[key] = `${value} — declared in ${rel}`;
      if (Object.keys(out).length >= limit) return out;
    }
  }
  return out;
}

/**
 * Trim a diff to fit a context window, keeping whole files.
 *
 * Jev's window is 32K tokens and a diff can dwarf that. Truncating mid-hunk
 * would hand the model a half-line of code and invite a confident answer about
 * something it cannot see, so this drops whole files at the boundary and
 * reports what it dropped — a report that silently reviewed 30% of a diff is
 * worse than one that says so.
 */
export function fitDiff(diff, maxChars = 60_000) {
  if (diff.length <= maxChars) return { diff, omitted: [] };

  const chunks = diff.split(/(?=^diff --git )/m);
  const kept = [];
  const omitted = [];
  let size = 0;

  for (const chunk of chunks) {
    if (size + chunk.length <= maxChars) {
      kept.push(chunk);
      size += chunk.length;
    } else {
      const name = /^diff --git a\/(\S+)/m.exec(chunk);
      omitted.push(name ? name[1] : "(unnamed)");
    }
  }
  return { diff: kept.join(""), omitted };
}

/**
 * Paths whose content is derived, opaque or platform-owned, so a reviewer
 * reading them learns nothing the product can act on. Capability skills and
 * agent files are rewritten by `fid upgrade` and AGENTS.md says not to edit
 * them, so a finding about one cannot be fixed by the product. The rest: `fid derive --check` already proves a derived file matches its
 * declaration, and a lockfile or a font is not prose.
 *
 * They are skipped by name and reported by name. The earlier behaviour was to
 * fill the budget in diff order and report whatever was left as "too large",
 * which dropped a page component to make room for a 3,000-line generated
 * module.
 */
export function isDerivedPath(path) {
  return (
    /(^|\/)generated\//.test(path) ||
    /(^|\/)(pnpm-lock\.yaml|package-lock\.json|yarn\.lock|Cargo\.lock|fiducial\.lock)$/.test(path) ||
    /\.(woff2?|ttf|otf|png|jpe?g|gif|webp|avif|ico|pdf|zip|min\.js|min\.css|map|tsbuildinfo)$/i.test(path) ||
    /(^|\/)(\.next|\.open-next|node_modules|dist)\//.test(path) ||
    /^\.(fiducial\/skills|claude\/skills|claude\/agents)\//.test(path)
  );
}

/**
 * Files `fiducial.lock` tracks whose content still hashes to what the platform
 * wrote.
 *
 * These are the platform's, not the product's: a capability script, a skill
 * file, a scaffolded config nobody has touched. Reviewing them reports on code
 * the product owner cannot change here, and it drowns the product's own files
 * — the first real run returned 88 findings, many of them about unmodified
 * template scripts. `fid derive --check` already proves they are intact, so
 * they are skipped by name. A file the product edits stops matching the hash
 * and is reviewed like any other.
 */
export function untouchedPlatformFiles(cwd) {
  let lock;
  try {
    lock = readFileSync(join(cwd, "fiducial.lock"), "utf8");
  } catch {
    return new Set();
  }
  const untouched = new Set();
  const re = /\[templates\."([^"]+)"\]\nsource_version = "[^"]*"\nhash = "([0-9a-f]+)"/g;
  for (const [, path, hash] of lock.matchAll(re)) {
    try {
      const bytes = readFileSync(join(cwd, path));
      if (createHash("sha256").update(bytes).digest("hex") === hash) untouched.add(path);
    } catch {
      continue;
    }
  }
  return untouched;
}

/**
 * Every path a pipeline declares as its output.
 *
 * `pipelines/*.toml` is where the platform already says which files are
 * derived. Reading it keeps this package from guessing: a reviewer told a
 * generated sitemap is "hand-written" is wrong, and `fid derive --check` is the
 * gate that proves it intact, so these are skipped by name.
 */
export function derivedOutputs(cwd) {
  const out = new Set();
  let names = [];
  try {
    names = readdirSync(join(cwd, "pipelines")).filter((n) => n.endsWith(".toml"));
  } catch {
    return out;
  }
  for (const name of names) {
    let text = "";
    try {
      text = readFileSync(join(cwd, "pipelines", name), "utf8");
    } catch {
      continue;
    }
    const block = /^outputs\s*=\s*\[([\s\S]*?)\]/m.exec(text);
    if (!block) continue;
    for (const [, path] of block[1].matchAll(/"([^"]+)"/g)) out.add(path);
  }
  return out;
}

/** Every tracked path, so a reviewer can tell "unchanged" from "missing". */
export function repoFiles(cwd, limit = 3000) {
  return (git(["ls-files"], cwd) ?? "").split("\n").filter(Boolean).slice(0, limit);
}

/**
 * Break a diff into structural units, each small enough to review alone.
 *
 * A unit is a file. A file larger than the budget is split at its hunk
 * boundaries, every piece keeping the file's header so it still says which file
 * it is. Small neighbouring files are then packed together by directory, so a
 * change that touches twenty tiny content files is one unit rather than twenty
 * requests, and two files in one directory are read side by side.
 *
 * Nothing is dropped to fit: a unit is added instead. `skipped` names what was
 * left out on purpose (see `isDerivedPath`) and why.
 */
export function splitDiff(
  diff,
  maxChars = 40_000,
  { tiny = 4_000, mergeTo = 12_000, skip = () => false } = {},
) {
  const files = diff.split(/(?=^diff --git )/m).filter((c) => c.trim());
  const skipped = [];
  const pieces = [];

  for (const chunk of files) {
    const name = /^diff --git a\/(\S+)/m.exec(chunk)?.[1] ?? "(unnamed)";
    if (isDerivedPath(name) || skip(name)) {
      skipped.push(name);
      continue;
    }
    if (chunk.length <= maxChars) {
      pieces.push({ file: name, diff: chunk });
      continue;
    }
    const head = chunk.slice(0, chunk.search(/^@@ /m) === -1 ? chunk.length : chunk.search(/^@@ /m));
    let current = head;
    let part = 1;
    for (const hunk of chunk.slice(head.length).split(/(?=^@@ )/m)) {
      if (current.length > head.length && current.length + hunk.length > maxChars) {
        pieces.push({ file: `${name} (part ${part++})`, diff: current });
        current = head;
      }
      current += hunk;
    }
    if (current.length > head.length) {
      pieces.push({ file: part > 1 ? `${name} (part ${part})` : name, diff: current });
    }
  }

  pieces.sort((a, b) => a.file.localeCompare(b.file));
  const dirOf = (file) => file.split("/").slice(0, -1).slice(0, 3).join("/");

  const units = [];
  for (const piece of pieces) {
    const last = units[units.length - 1];
    if (
      last &&
      last.dir === dirOf(piece.file) &&
      last.diff.length + piece.diff.length <= maxChars
    ) {
      last.diff += piece.diff;
      last.files.push(piece.file);
    } else {
      units.push({ dir: dirOf(piece.file), files: [piece.file], diff: piece.diff });
    }
  }

  // A change of many tiny files in many directories would otherwise be one
  // request each. Units under `tiny` characters are packed with their
  // neighbours, across directories, up to `mergeTo` — still never past
  // `maxChars`, and a unit that is not tiny is left alone.
  const packed = [];
  for (const unit of units) {
    const last = packed[packed.length - 1];
    if (
      last &&
      last.diff.length < tiny &&
      unit.diff.length < tiny &&
      last.diff.length + unit.diff.length <= Math.min(mergeTo, maxChars)
    ) {
      last.diff += unit.diff;
      last.files.push(...unit.files);
      last.dir = "";
    } else {
      packed.push({ ...unit, files: [...unit.files] });
    }
  }

  return {
    units: packed.map((u, i) => ({
      id: i + 1,
      label: u.files.length === 1 ? u.files[0] : `${u.dir || "mixed"}${u.dir ? "/" : ""} (${u.files.length} files)`,
      files: u.files,
      diff: u.diff,
    })),
    skipped,
  };
}

/**
 * The rules a change is judged against: the repository's own, verbatim.
 *
 * `AGENTS.md` is the agent context and `MISSION.md` is why it is shaped that
 * way. They are read from the working tree rather than summarised here, because
 * a summary is a second copy of the rules and the first thing to drift from
 * them. Each is capped so one enormous file cannot crowd out the change.
 */
export function collectRules(cwd, maxChars = 40_000) {
  const rules = {};
  for (const name of ["AGENTS.md", "MISSION.md"]) {
    const text = readIfText(cwd, name, 400_000);
    if (text?.trim()) rules[name] = text.length > maxChars ? text.slice(0, maxChars) : text;
  }
  return rules;
}

/** Documents that are written once and never edited: decisions are append-only. */
const APPEND_ONLY = /(^|\/)docs\/specs\//;
const NOT_DOCS = /(^|\/)(CHANGELOG\.md|LICENSE\.md|node_modules\/|docs\/paper\/|docs\/prose\.lock)/;
const GENERIC_NAMES = new Set([
  "index", "page", "layout", "readme", "main", "lib", "utils", "util", "types",
  "config", "mod", "test", "tests", "style", "styles", "global", "globals",
]);

/**
 * Which documents may have gone stale, found by name rather than by hope.
 *
 * Documentation drifts because nothing connects an edit to the sentence that
 * describes it. This is the connection, made cheaply and over-inclusively: a
 * document that was not changed but names a file the change touched is a
 * candidate, and so is a flag the change adds that no document mentions. The
 * model then reads each against the diff and reports only the ones that are
 * actually wrong, which is the same split every other check here uses.
 *
 * `docs/specs/` is excluded from the stale list on purpose. A decision there is
 * a record of what was believed, so the right response to a change is a new
 * dated decision, not an edit.
 */
export function docsDrift(cwd, files, diff, { limit = 30, skip = () => false } = {}) {
  const tracked = (git(["ls-files", "*.md", "**/*.md"], cwd) ?? "").split("\n").filter(Boolean);
  const docs = [...new Set(tracked)].filter((d) => !NOT_DOCS.test(d));
  const changed = new Set(files);

  const sources = files.filter((f) => !f.endsWith(".md") && !isDerivedPath(f) && !skip(f));
  const tokens = new Set();
  for (const f of sources) {
    tokens.add(f);
    const base = f.split("/").pop().replace(/\.[^.]+$/, "");
    if (base.length >= 6 && !GENERIC_NAMES.has(base.toLowerCase())) tokens.add(base);
  }

  const stale = [];
  const corpus = [];
  for (const doc of docs) {
    const text = readIfText(cwd, doc, 400_000);
    if (text === null) continue;
    corpus.push(text);
    if (changed.has(doc) || APPEND_ONLY.test(doc)) continue;
    const lines = text.split("\n");
    let hits = 0;
    for (let i = 0; i < lines.length && hits < 3; i++) {
      const token = [...tokens].find((t) => lines[i].includes(t));
      if (token) {
        stale.push({ doc, line: i + 1, token });
        hits++;
      }
    }
  }

  const joined = corpus.join("\n");
  const undocumented = [];
  for (const { file, text } of addedLines(diff)) {
    if (!file || file.endsWith(".md") || isDerivedPath(file) || skip(file)) continue;
    for (const flag of text.match(/--[a-z][a-z0-9]+(?:-[a-z0-9]+)*/g) ?? []) {
      if (!joined.includes(flag) && !undocumented.some((u) => u.token === flag)) {
        undocumented.push({ token: flag, file });
      }
    }
  }

  const docChanged = files.some((f) => f.endsWith(".md") && !APPEND_ONLY.test(f));
  return {
    stale: stale.slice(0, limit),
    undocumented: undocumented.slice(0, 15),
    noDocsChanged: sources.length > 0 && !docChanged,
  };
}

/** Files with their added and removed line counts, for the cross-cutting pass. */
export function diffStat(diff) {
  const stat = [];
  for (const chunk of diff.split(/(?=^diff --git )/m)) {
    const name = /^diff --git a\/(\S+)/m.exec(chunk)?.[1];
    if (!name) continue;
    let add = 0;
    let del = 0;
    for (const line of chunk.split("\n")) {
      if (line.startsWith("+") && !line.startsWith("+++")) add++;
      else if (line.startsWith("-") && !line.startsWith("---")) del++;
    }
    stat.push({ file: name, add, del });
  }
  return stat;
}

/** Run `fn` over `items`, at most `limit` at a time, keeping result order. */
export async function mapLimit(items, limit, fn) {
  const results = new Array(items.length);
  let next = 0;
  async function worker() {
    while (next < items.length) {
      const i = next++;
      results[i] = await fn(items[i], i);
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

/**
 * The declared thesis, read through `fid` rather than re-parsed here.
 *
 * `fid thesis --json` already resolves which entry is current, which fields are
 * unanswered, and what supersedes what — with the tests behind it. Reading
 * `thesis.toml` again in JavaScript would be a second parser for one
 * declaration, and the two would disagree the first time the schema moved.
 *
 * Returns `null` when there is no thesis, no `fid` on PATH, or a `fid` too old
 * to know the flag. Every one of those is "nothing to advise about" rather than
 * an error: this runs in a hook, and a product without a thesis is a normal
 * product.
 */
export function collectThesis(cwd) {
  let raw;
  try {
    raw = execFileSync("fid", ["thesis", "--json"], {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed?.declared ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * Hand-written copy that restates the claim, with where each piece came from.
 *
 * This is the mechanical half of `copy_contradicts_the_claim` and
 * `overclaims_against_evidence`. It is deliberately over-inclusive and shallow:
 * finding the sentences a product sells itself with is a fixed set of
 * conventional places, and deciding whether one of them contradicts the thesis
 * is the judgment.
 *
 * The places, and why each:
 *
 * - `README.md`'s opening prose — the first thing a reader meets.
 * - `meta.description` in every `messages/*.json` — what a search result shows,
 *   and in the first product's case the file that restates the thesis as "A meter measures
 *   every reading" with nothing connecting the two.
 * - `MISSION.md`'s opening — where the claim used to live, and where it drifts
 *   back to.
 *
 * Every entry is labelled with its file so a finding can point at a real line
 * instead of describing something the model believes it saw.
 */
export function restatements(cwd, limit = 12) {
  const out = [];

  const readIf = (rel) => {
    try {
      const p = join(cwd, rel);
      if (!statSync(p).isFile()) return null;
      return readFileSync(p, "utf8");
    } catch {
      return null;
    }
  };

  // The first real paragraph of a Markdown file: past the H1, past badges,
  // blockquotes and HTML comments, stopping at the first blank line after it.
  const openingProse = (text) => {
    const lines = text.split("\n");
    const para = [];
    for (const line of lines) {
      const t = line.trim();
      if (!para.length) {
        if (!t) continue;
        if (t.startsWith("#") || t.startsWith("<!--") || t.startsWith("[!")) continue;
        if (t.startsWith("![") || t.startsWith("---")) continue;
        para.push(t.replace(/^>\s*/, ""));
        continue;
      }
      if (!t) break;
      para.push(t.replace(/^>\s*/, ""));
    }
    const joined = para.join(" ").trim();
    return joined.length > 20 ? joined.slice(0, 600) : null;
  };

  for (const rel of ["README.md", "MISSION.md"]) {
    const text = readIf(rel);
    if (!text) continue;
    const prose = openingProse(text);
    if (prose) out.push(`${rel} (opening): ${prose}`);
  }

  const catalogs = (git(["ls-files", "messages/*.json", "**/messages/*.json"], cwd) ?? "")
    .split("\n")
    .filter(Boolean)
    .slice(0, 8);

  for (const rel of catalogs) {
    const text = readIf(rel);
    if (!text) continue;
    let description;
    try {
      description = JSON.parse(text)?.meta?.description;
    } catch {
      continue;
    }
    if (typeof description === "string" && description.trim()) {
      out.push(`${rel} (meta.description): ${description.trim().slice(0, 600)}`);
    }
  }

  return out.slice(0, limit);
}

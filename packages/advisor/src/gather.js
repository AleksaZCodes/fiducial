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
import { readFileSync, statSync } from "node:fs";
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
 *   and in fon's case the file that restates the thesis as "A person confirms
 *   every event" with nothing connecting the two.
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
